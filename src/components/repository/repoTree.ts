// =====================================================================
// 本地仓库目录树构建。
//
// 把扁平的 LocalRepository[] 按各自 localPath 的文件系统目录层级，
// 组织成「森林」（可能多棵树），供 LocalRepoTable.vue 的 el-table
// 树形表格消费。核心规则（与计划文档一致）：
//   - 树根取各仓库路径的最长公共前缀；不同盘符 / 无公共前缀 → 多棵树；
//   - 连续只有单一子目录的中间层压缩合并，减少空层级；
//   - 目录节点聚合其子树的仓库数与各状态计数，供勾选级联与汇总展示。
//
// 嵌套仓库的处理：当某目录本身是 git 工程、其子目录里又有别的 git 工程时，
// 该路径会合并成「单个仓库节点」——它携带自身 repo（按仓库展示），同时
// 带 children（其下的子仓库），避免同一路径既出现仓库行又出现目录行。
// 实现上：用「含仓库段在内的完整路径」逐级建节点，并在仓库路径节点挂 repo，
// 于是目录与仓库落到同一节点；单链压缩刻意避开「本身是仓库」的节点。
//
// 本模块为纯函数、无 Vue 依赖，便于独立推理与日后补测。
// =====================================================================

import type { LocalRepository, RepositoryStatus } from '@/types/repository';

/** 6 个状态键全列的计数表（缺省 0）。 */
export type StatusSummary = Record<RepositoryStatus, number>;

/** 目录节点：可展开的父行，聚合其子树的仓库数与状态统计。 */
export interface DirNode {
  type: 'dir';
  /** el-table row-key：'dir:<fullPath>'，全树唯一。 */
  id: string;
  /** 首列展示名；压缩后可能是多段拼接（如 code/work）。 */
  label: string;
  /** 合并后的完整绝对路径。 */
  fullPath: string;
  /** 子树仓库总数。 */
  repoCount: number;
  /** 子树按状态计数。 */
  summary: StatusSummary;
  children: RepoTreeNode[];
}

/** 仓库节点：对应一个真实 LocalRepository；可能同时是含子仓库的目录。 */
export interface RepoLeaf {
  type: 'repo';
  /** el-table row-key：'repo:<repo.id>'。 */
  id: string;
  /** 仓库末段目录名。 */
  label: string;
  /** 原始仓库对象，操作列与选择映射都回到它。 */
  repo: LocalRepository;
  /** 合并节点：该仓库目录下还有子仓库时携带；普通仓库无此字段（故不渲染展开箭头）。 */
  children?: RepoTreeNode[];
  /** 子仓库数（不含自身），用于「含 N 个子仓库」标注。 */
  childRepoCount?: number;
}

/** 树节点 = 目录节点 | 仓库节点。 */
export type RepoTreeNode = DirNode | RepoLeaf;

// 固定的状态键顺序，用于初始化与遍历 summary（与 RepositoryStatus 对齐）。
const STATUS_KEYS: readonly RepositoryStatus[] = [
  'clean',
  'dirty',
  'ahead',
  'behind',
  'diverged',
  'unknown',
];

/** 生成一张全 0 的状态计数表。 */
function emptySummary(): StatusSummary {
  return { clean: 0, dirty: 0, ahead: 0, behind: 0, diverged: 0, unknown: 0 };
}

// 单条路径的解析结果：根标识 + 分隔符 + 去空后的逐级目录段。
interface ParsedPath {
  /** 同一根标识的仓库归入同一棵树（POSIX 根 / 盘符 / UNC 主机+共享）。 */
  rootKey: string;
  /** 该路径重组时使用的分隔符（Windows '\\'、POSIX '/'）。 */
  sep: string;
  /** 去掉根之后的逐级目录段，最后一段是仓库目录名。 */
  segments: string[];
}

/**
 * 解析绝对路径为「根标识 + 分隔符 + 路径段」。
 * 兼容 POSIX（/a/b）与 Windows（C:\a\b、\\srv\share\a）两种风格。
 */
function parsePath(p: string): ParsedPath {
  // 含反斜杠或盘符前缀 → 判定为 Windows 风格，否则按 POSIX 处理
  const isWindows = /\\/.test(p) || /^[A-Za-z]:/.test(p);
  if (!isWindows) {
    // POSIX：根标识统一为 '/'，其余为去空后的段
    return { rootKey: '/', sep: '/', segments: p.split('/').filter(Boolean) };
  }

  const sep = '\\';
  // UNC 路径 \\server\share\...：根标识取 \\server\share
  const unc = p.match(/^\\\\([^\\/]+)[\\/]([^\\/]+)(.*)$/);
  if (unc) {
    const rest = unc[3].split(/[\\/]+/).filter(Boolean);
    return { rootKey: `\\\\${unc[1]}\\${unc[2]}`, sep, segments: rest };
  }
  // 盘符路径 C:\...：根标识取盘符（大写归一）
  const drive = p.match(/^([A-Za-z]):(.*)$/);
  if (drive) {
    const rest = drive[2].split(/[\\/]+/).filter(Boolean);
    return { rootKey: `${drive[1].toUpperCase()}:`, sep, segments: rest };
  }
  // 兜底：无盘符的反斜杠异常路径，根标识置空，整体切分
  return { rootKey: '', sep, segments: p.split(/[\\/]+/).filter(Boolean) };
}

/** 把根标识与若干段拼成完整路径（区分 POSIX 与 Windows 拼接规则）。 */
function joinPath(rootKey: string, sep: string, segs: string[]): string {
  if (rootKey === '/') {
    // POSIX：'/' 直接前缀，根本身（segs 为空）即 '/'
    return `/${segs.join('/')}`;
  }
  // Windows 盘符 / UNC：段非空时用 sep 衔接，否则就是根标识本身
  return segs.length > 0 ? `${rootKey}${sep}${segs.join(sep)}` : rootKey;
}

/** 在已有完整路径后追加一段子目录。 */
function appendSeg(parentFull: string, sep: string, seg: string): string {
  // POSIX 根为 '/'，已以分隔符结尾，避免拼出双斜杠
  return parentFull.endsWith(sep) ? `${parentFull}${seg}` : `${parentFull}${sep}${seg}`;
}

/** 求若干段序列的最长公共前缀。 */
function longestCommonPrefix(list: string[][]): string[] {
  if (list.length === 0) return [];
  // 以第一条为基准，逐条收缩公共长度
  let prefix = list[0].slice();
  for (let i = 1; i < list.length; i += 1) {
    const cur = list[i];
    let j = 0;
    while (j < prefix.length && j < cur.length && prefix[j] === cur[j]) j += 1;
    prefix = prefix.slice(0, j);
    if (prefix.length === 0) break;
  }
  return prefix;
}

// 桶内单条记录：解析出的段 + 原始仓库。
interface BucketEntry {
  segments: string[];
  repo: LocalRepository;
}

// 可变中间节点：按路径逐级搭建，repo 表示该路径是否为仓库。
interface MutNode {
  fullPath: string;
  label: string;
  repo?: LocalRepository;
  children: MutNode[];
}

/** 由同一根标识下的若干仓库构建一棵可变目录树（含仓库段在内逐级建节点）。 */
function buildBucket(rootKey: string, sep: string, entries: BucketEntry[]): MutNode {
  // 公共前缀基于「父目录段」计算：仓库名本身不参与，保证根是纯目录
  const parentSegsList = entries.map((e) => e.segments.slice(0, -1));
  const rootSegs = longestCommonPrefix(parentSegsList);

  const rootFull = joinPath(rootKey, sep, rootSegs);
  // 根节点 label 用完整路径，让用户一眼看到这棵树的物理位置
  const root: MutNode = { fullPath: rootFull, label: rootFull, children: [] };

  for (const { segments, repo } of entries) {
    // 含仓库段在内：从根逐级 find-or-create，最末（仓库段）节点挂 repo。
    // 若该路径已因某个子仓库被建为中间节点，repo 会自然落到同一节点 → 合并。
    const relSegs = segments.slice(rootSegs.length);
    let cur = root;
    for (const seg of relSegs) {
      let child = cur.children.find((c) => c.label === seg);
      if (!child) {
        child = { fullPath: appendSeg(cur.fullPath, sep, seg), label: seg, children: [] };
        cur.children.push(child);
      }
      cur = child;
    }
    cur.repo = repo;
  }
  return root;
}

/** 压缩单链目录：连续只有一个「纯目录」子节点的层级合并成一行。 */
function compress(node: MutNode, sep: string): void {
  // 先递归压缩所有子节点
  for (const c of node.children) compress(c, sep);
  // 反复合并：本节点非仓库、且唯一子节点也非仓库时才并（仓库节点须独立成行）
  for (;;) {
    if (node.repo) break;
    if (node.children.length !== 1) break;
    const only = node.children[0];
    if (only.repo) break;
    node.label = `${node.label}${sep}${only.label}`;
    node.fullPath = only.fullPath;
    node.children = only.children;
  }
}

// 转换结果：输出节点 + 其子树仓库数（含自身）+ 状态汇总（含自身）。
interface Converted {
  node: RepoTreeNode;
  count: number;
  summary: StatusSummary;
}

/** 后序遍历：把可变节点转成输出节点，并自底向上聚合仓库数与状态统计。 */
function convert(m: MutNode): Converted {
  const childResults = m.children.map(convert);

  // 先汇总所有子树
  const summary = emptySummary();
  let count = 0;
  for (const r of childResults) {
    count += r.count;
    for (const k of STATUS_KEYS) summary[k] += r.summary[k];
  }
  // 自身若是仓库，计入统计
  if (m.repo) {
    count += 1;
    summary[m.repo.status] += 1;
  }

  const childNodes = childResults.map((r) => r.node);

  if (m.repo) {
    // 仓库节点（可能同时含子仓库）
    const node: RepoLeaf = {
      type: 'repo',
      id: `repo:${m.repo.id}`,
      label: m.label,
      repo: m.repo,
    };
    if (childNodes.length > 0) {
      node.children = childNodes;
      // 子仓库数 = 子树总数减去自身
      node.childRepoCount = count - 1;
    }
    return { node, count, summary };
  }

  // 纯目录节点（自身非仓库，repoCount 即子树仓库数）
  const node: DirNode = {
    type: 'dir',
    id: `dir:${m.fullPath}`,
    label: m.label,
    fullPath: m.fullPath,
    repoCount: count,
    summary,
    children: childNodes,
  };
  return { node, count, summary };
}

/** 取节点的子节点列表（目录必有 children，仓库节点可选）。 */
function childrenOf(node: RepoTreeNode): RepoTreeNode[] {
  return node.type === 'dir' ? node.children : (node.children ?? []);
}

/** 排序：目录在前、仓库在后，同类按名称不分大小写升序。 */
function sortTree(nodes: RepoTreeNode[]): void {
  nodes.sort((a, b) => {
    if (a.type !== b.type) return a.type === 'dir' ? -1 : 1;
    return a.label.localeCompare(b.label);
  });
  for (const n of nodes) sortTree(childrenOf(n));
}

/** 由扁平仓库列表构建目录森林。空列表→[]；无公共前缀 / 多盘符→多棵树。 */
export function buildRepoForest(repos: LocalRepository[]): RepoTreeNode[] {
  if (repos.length === 0) return [];

  // 1. 解析每条路径，按「根标识」分桶（不同根天然分成不同树）；
  //    分隔符随桶一并记录，避免把它编码进 key 带来的解析歧义。
  const buckets = new Map<string, { sep: string; entries: BucketEntry[] }>();
  for (const repo of repos) {
    const { rootKey, sep, segments } = parsePath(repo.localPath);
    const bucket = buckets.get(rootKey) ?? { sep, entries: [] };
    bucket.entries.push({ segments, repo });
    buckets.set(rootKey, bucket);
  }

  // 2. 每个桶构建一棵树、压缩单链，再转换为输出节点
  const forest: RepoTreeNode[] = [];
  for (const [rootKey, { sep, entries }] of buckets) {
    const root = buildBucket(rootKey, sep, entries);
    compress(root, sep);
    forest.push(convert(root).node);
  }

  // 3. 排序
  sortTree(forest);
  return forest;
}

/** 抽取目录状态统计中计数大于 0 的项，按固定顺序输出，供紧凑展示。 */
export function summaryParts(
  summary: StatusSummary,
): { status: RepositoryStatus; count: number }[] {
  return STATUS_KEYS.filter((k) => summary[k] > 0).map((k) => ({ status: k, count: summary[k] }));
}
