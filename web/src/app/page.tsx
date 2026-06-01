const API_BASE =
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:3001/api/v1";

async function fetchHealth(): Promise<{ status?: string }> {
  try {
    const res = await fetch(`${API_BASE}/health`, { cache: "no-store" });
    return res.json();
  } catch {
    return { status: "unreachable" };
  }
}

export default async function HomePage() {
  const health = await fetchHealth();

  return (
    <main className="mx-auto flex min-h-screen max-w-3xl flex-col gap-8 px-6 py-16">
      <header>
        <h1 className="text-3xl font-bold tracking-tight">灵犀节点 SparkNode</h1>
        <p className="mt-2 text-neutral-400">
          高维叙事与世界演算引擎 · 最小可用前端入口
        </p>
      </header>

      <section className="rounded-lg border border-neutral-800 bg-neutral-950 p-6">
        <h2 className="text-lg font-semibold">API 网关状态</h2>
        <p className="mt-2 font-mono text-sm text-emerald-400">
          {health.status ?? "unknown"}
        </p>
        <p className="mt-4 text-sm text-neutral-500">
          后端地址：<code className="text-neutral-300">{API_BASE}</code>
        </p>
      </section>

      <section className="rounded-lg border border-neutral-800 p-6">
        <h2 className="text-lg font-semibold">快速操作</h2>
        <ul className="mt-3 list-inside list-disc space-y-2 text-sm text-neutral-300">
          <li>
            建立故事：<code>POST {API_BASE}/stories</code>
          </li>
          <li>
            取得 DAG：<code>GET {API_BASE}/stories/:id/dag</code>
          </li>
          <li>
            觀察者坍縮：<code>POST {API_BASE}/:storyId/collapse</code>
          </li>
          <li>
            灵犀矩阵：<code>POST {API_BASE}/generate/compare</code>
          </li>
        </ul>
      </section>
    </main>
  );
}
