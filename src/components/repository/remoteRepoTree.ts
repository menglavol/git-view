// =====================================================================
// 远程仓库树构建。
//
// 把扁平的 RemoteRepository[] 按「平台 → 命名空间路径」组织成森林，供
// RemoteRepoTree.vue 的 el-table 树形表格消费。核心规则：
//   - 顶层按平台分组（GitHub / GitLab / Gitee），每个平台一棵树的根；
//   - 平台组内按 fullName 的 '/' 层级建目录节点，末段挂仓库（叶子）；
//     GitHub 通常是 owner/repo（两段），GitLab 可有多级 group/subgroup/project；
//   - 连续只有单一子目录的中间层压缩合并，减少空层级（平台根与仓库节点不参与）；
//   - 目录/平台节点聚合其子树的仓库数，供展开提示与汇总展示。
//
// 与本地仓库树（repoTree.ts）的差异：本地按文件系统路径（含盘符/分隔符），
// 远程统一按 fullName 的 '/' 分段，且多一层「平台」分组、无工作区状态汇总。
//
// 本模块为纯函数、无 Vue 依赖，便于独立推理与日后补测。
// =====================================================================

import type { RemoteRepository } from '@/types/repository';
import type { GitPlatform } from '@/types/account';

/** 平台根的固定输出顺序。 */
const PLATFORM_ORDER: readonly GitPlatform[] = ['github', 'gitlab', 'gitee'];

/** 平台到展示名的映射（平台根节点的 label）。 */
const PLATFORM_LABEL: Record<GitPlatform, string> = {
  github: 'GitHub',
  gitlab: 'GitLab',
  gitee: 'Gitee',
};

/** 目录节点：可展开的父行（平台根或命名空间目录），聚合子树仓库数。 */
export interface RemoteDirNode {
  type: 'dir';
  /** el-table row-key：平台根 'platform:<p>'，目录 '<parentId>/<seg>'，全树唯一。 */
  id: string;
  /** 首列展示名；压缩后可能是多段拼接（如 group/subgroup）。 */
  label: string;
  /** 子树仓库总数。 */
  repoCount: number;
  /** 是否为平台根节点（用于加粗样式，且不参与单链压缩）。 */
  isPlatform: boolean;
  children: RemoteTreeNode[];
}

/** 仓库节点：对应一个真实 RemoteRepository，远程树中一定是叶子。 */
export interface RemoteRepoLeaf {
  type: 'repo';
  /** el-table row-key：'repo:<repo.id>'。 */
  id: string;
  /** 仓库末段名（fullName 最后一段，通常等于 repo.name）。 */
  label: string;
  /** 原始仓库对象，操作列与选择映射都回到它。 */
  repo: RemoteRepository;
}

/** 树节点 = 目录节点 | 仓库节点。 */
export type RemoteTreeNode = RemoteDirNode | RemoteRepoLeaf;

// 可变中间节点：按路径逐级搭建，repo 表示该节点是仓库叶子。
interface MutNode {
  id: string;
  label: string;
  repo?: RemoteRepository;
  isPlatform: boolean;
  children: MutNode[];
}

/** 在父节点下按 label 查找已有的目录子节点，没有则新建并挂上。 */
function findOrCreateDir(parent: MutNode, label: string): MutNode {
  // 只匹配纯目录子节点（带 repo 的是仓库叶子，不可复用为目录）
  let child = parent.children.find((c) => c.label === label && !c.repo);
  if (!child) {
    child = { id: `${parent.id}/${label}`, label, isPlatform: false, children: [] };
    parent.children.push(child);
  }
  return child;
}

/** 压缩单链目录：连续只有一个「纯目录」子节点的层级合并成一行。 */
function compress(node: MutNode): void {
  // 先递归压缩所有子节点
  for (const c of node.children) compress(c);
  // 反复合并：平台根与仓库节点不参与；唯一子节点必须也是纯目录才并
  for (;;) {
    if (node.repo || node.isPlatform) break;
    if (node.children.length !== 1) break;
    const only = node.children[0];
    if (only.repo) break;
    node.label = `${node.label}/${only.label}`;
    node.id = only.id;
    node.children = only.children;
  }
}

// 转换结果：输出节点 + 其子树仓库数（含自身）。
interface Converted {
  node: RemoteTreeNode;
  count: number;
}

/** 后序遍历：把可变节点转成输出节点，并自底向上聚合仓库数。 */
function convert(m: MutNode): Converted {
  const childResults = m.children.map(convert);
  let count = 0;
  for (const r of childResults) count += r.count;
  if (m.repo) count += 1;

  // 仓库叶子（远程树里仓库一定无子节点）
  if (m.repo) {
    const node: RemoteRepoLeaf = {
      type: 'repo',
      id: m.id,
      label: m.label,
      repo: m.repo,
    };
    return { node, count };
  }

  // 目录 / 平台节点
  const node: RemoteDirNode = {
    type: 'dir',
    id: m.id,
    label: m.label,
    repoCount: count,
    isPlatform: m.isPlatform,
    children: childResults.map((r) => r.node),
  };
  return { node, count };
}

/** 排序：目录在前、仓库在后，同类按名称不分大小写升序。 */
function sortNodes(nodes: RemoteTreeNode[]): void {
  nodes.sort((a, b) => {
    if (a.type !== b.type) return a.type === 'dir' ? -1 : 1;
    return a.label.localeCompare(b.label);
  });
  for (const n of nodes) {
    if (n.type === 'dir') sortNodes(n.children);
  }
}

/**
 * 由扁平远程仓库列表构建森林：顶层平台节点，其下按 fullName 命名空间层级展开。
 * 空列表 → []；只输出实际存在的平台，顺序固定为 GitHub → GitLab → Gitee。
 */
export function buildRemoteForest(repos: RemoteRepository[]): RemoteTreeNode[] {
  if (repos.length === 0) return [];

  // 1. 按平台分桶，桶内按 fullName 逐级建目录、末段挂仓库
  const platformRoots = new Map<GitPlatform, MutNode>();
  for (const repo of repos) {
    let root = platformRoots.get(repo.platform);
    if (!root) {
      root = {
        id: `platform:${repo.platform}`,
        label: PLATFORM_LABEL[repo.platform],
        isPlatform: true,
        children: [],
      };
      platformRoots.set(repo.platform, root);
    }

    // fullName 分段：前面是命名空间目录，最后一段是仓库；异常空值回退仓库名
    const segments = repo.fullName.split('/').filter(Boolean);
    if (segments.length === 0) segments.push(repo.name);

    let cur = root;
    for (let i = 0; i < segments.length - 1; i += 1) {
      cur = findOrCreateDir(cur, segments[i]);
    }
    cur.children.push({
      id: `repo:${repo.id}`,
      label: segments[segments.length - 1],
      repo,
      isPlatform: false,
      children: [],
    });
  }

  // 2. 按固定平台顺序压缩、转换、排序后输出
  const forest: RemoteTreeNode[] = [];
  for (const platform of PLATFORM_ORDER) {
    const root = platformRoots.get(platform);
    if (!root) continue;
    compress(root);
    const node = convert(root).node;
    // 平台根内部排序（顶层平台顺序保持 PLATFORM_ORDER，不再重排）
    if (node.type === 'dir') sortNodes(node.children);
    forest.push(node);
  }
  return forest;
}
