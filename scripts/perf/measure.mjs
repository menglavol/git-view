// =====================================================================
// 性能基线测量脚本（spec SC-006 / SC-007 数据层部分）
//
// 说明：本脚本用 Node 直接对 SQLite 跑代表性查询并计时，量化「数据层」开销
// （搜索 / 筛选 / 分页 SQL）。UI 渲染流畅度、批量 Clone 总耗时（SC-005/008）
// 涉及前端与子进程，需在应用内手动测量并记入 perf-baseline.md。
//
// 之所以用 .mjs 而非前端 src 下的 .ts：它是与 seed-data.sh 并列的一次性辅助
// 工具，不属于应用源码，因此不纳入前端 lint / 类型 / 注释门禁范围。
//
// 前提：已运行 seed-data.sh 写入种子数据，且系统安装了 sqlite3。
// 路径：默认各平台标准位置，可用环境变量 GITVIEW_DB 覆盖。
//
// 调用：node scripts/perf/measure.mjs
// =====================================================================

import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

// 定位数据库：优先环境变量，其次按平台推断标准位置
function detectDb() {
  if (process.env.GITVIEW_DB) return process.env.GITVIEW_DB;
  const home = homedir();
  switch (process.platform) {
    case 'darwin':
      return join(home, 'Library', 'Application Support', 'com.gitview.app', 'gitview.db');
    case 'win32':
      return join(process.env.LOCALAPPDATA ?? home, 'com.gitview.app', 'gitview.db');
    default:
      // Linux：遵循 XDG 规范
      return join(process.env.XDG_DATA_HOME ?? join(home, '.local', 'share'), 'com.gitview.app', 'gitview.db');
  }
}

const dbPath = detectDb();

// 数据库不存在时直接退出：避免误以为「0ms = 极快」
if (!existsSync(dbPath)) {
  console.error(`未找到数据库：${dbPath}`);
  console.error('请先正常启动一次应用建表，再运行 seed-data.sh，最后跑本脚本。');
  process.exit(1);
}

// 执行单条 SQL 并返回耗时（毫秒）；用 sqlite3 CLI 避免引入原生依赖
function timeQuery(label, sql) {
  const start = performance.now();
  try {
    execFileSync('sqlite3', [dbPath, sql], { encoding: 'utf8' });
  } catch (e) {
    console.error(`  ✗ ${label} 查询失败：${e instanceof Error ? e.message : String(e)}`);
    return;
  }
  const ms = (performance.now() - start).toFixed(1);
  console.log(`  ${label.padEnd(28)} ${ms} ms`);
}

console.log(`→ 数据层查询基线（DB=${dbPath}）`);

// SC-006：远程仓库搜索（name/description 模糊匹配 + 排序）
timeQuery(
  '远程仓库搜索(LIKE)',
  "SELECT id FROM remote_repositories WHERE name LIKE '%repo-1%' OR description LIKE '%repo-1%' ORDER BY synced_at DESC;",
);

// SC-006：远程仓库按收藏筛选
timeQuery('远程仓库筛选(收藏)', 'SELECT id FROM remote_repositories WHERE is_favorite = 1;');

// SC-005：本地仓库按状态筛选
timeQuery('本地仓库筛选(状态)', "SELECT id FROM local_repositories WHERE status = 'dirty';");

// SC-007：操作日志分页（最近 50 条）
timeQuery(
  '操作日志分页(50)',
  'SELECT id FROM operation_logs ORDER BY occurred_at DESC LIMIT 50 OFFSET 0;',
);

// SC-007：操作日志按类型筛选计数
timeQuery('操作日志筛选(类型)', "SELECT count(*) FROM operation_logs WHERE operation_type = 'push';");

console.log('\n提示：UI 渲染流畅度与批量 Clone 总耗时请在应用内手动测量，记入 perf-baseline.md。');
