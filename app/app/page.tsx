import Link from "next/link";

const HEALTHY_MAX_AGE_MS = 30_000;

type WorkerInfo = {
  worker_id: string;
  multiaddr: string;
  last_seen: number;
};

type ManifestSegment = {
  segment_index: number;
  plaintext_len: number;
  ciphertext_len: number;
  nonce: number[];
};

type SignedManifest = {
  manifest: {
    file_id: string;
    original_len: number;
    original_hash_hex: string;
    segments: ManifestSegment[];
  };
  uploader_peer_id: string;
  uploader_public_key_protobuf: number[];
  signature: number[];
};

type ShardRecord = {
  worker_id: string;
  worker_multiaddr: string;
  file_id: string;
  segment_index: number;
  shard_index: number;
  shard_hash_hex: string;
};

type LocateResp = {
  file_id: string;
  shards: ShardRecord[];
};

type DashboardPageProps = {
  searchParams: Promise<{ file_id?: string }>;
};

export const dynamic = "force-dynamic";

function satelliteBaseUrl(): string {
  return process.env.SATELLITE_URL?.trim() || "http://127.0.0.1:7070";
}

async function fetchJson<T>(url: string): Promise<T> {
  const res = await fetch(url, { cache: "no-store" });
  if (!res.ok) {
    let message = `${res.status} ${res.statusText}`;
    try {
      const body = await res.text();
      if (body) message = `${message} - ${body}`;
    } catch {
      // ignore body parse failures
    }
    throw new Error(message);
  }
  return (await res.json()) as T;
}

function fmtAge(ms: number): string {
  const secs = Math.max(0, Math.floor(ms / 1000));
  return `${secs}s lag`;
}

export default async function Home({ searchParams }: DashboardPageProps) {
  const params = await searchParams;
  const fileId = (params.file_id || "").trim();
  const base = satelliteBaseUrl();

  let workers: WorkerInfo[] = [];
  let workersError: string | null = null;
  try {
    workers = await fetchJson<WorkerInfo[]>(`${base}/workers`);
  } catch (err) {
    workersError = err instanceof Error ? err.message : String(err);
  }

  const newestLastSeen = workers.reduce((max, w) => Math.max(max, Number(w.last_seen) || 0), 0);

  let manifest: SignedManifest | null = null;
  let locate: LocateResp | null = null;
  let fileError: string | null = null;

  if (fileId) {
    try {
      const [manifestRes, locateRes] = await Promise.all([
        fetch(`${base}/manifest?file_id=${encodeURIComponent(fileId)}`, { cache: "no-store" }),
        fetchJson<LocateResp>(`${base}/locate?file_id=${encodeURIComponent(fileId)}`),
      ]);

      if (manifestRes.ok) {
        manifest = (await manifestRes.json()) as SignedManifest;
      } else if (manifestRes.status !== 404) {
        const t = await manifestRes.text();
        fileError = `manifest lookup failed: ${manifestRes.status} ${manifestRes.statusText}${
          t ? ` - ${t}` : ""
        }`;
      }

      locate = locateRes;
    } catch (err) {
      fileError = err instanceof Error ? err.message : String(err);
    }
  }

  const replicaMap = new Map<string, Set<string>>();
  for (const rec of locate?.shards || []) {
    const key = `${rec.segment_index}:${rec.shard_index}`;
    if (!replicaMap.has(key)) replicaMap.set(key, new Set<string>());
    replicaMap.get(key)!.add(rec.worker_id);
  }
  const replicaCounts = Array.from(replicaMap.values()).map((s) => s.size);
  const uniqueShards = replicaCounts.length;
  const minReplicas = replicaCounts.length ? Math.min(...replicaCounts) : 0;
  const maxReplicas = replicaCounts.length ? Math.max(...replicaCounts) : 0;
  const avgReplicas = replicaCounts.length
    ? (replicaCounts.reduce((a, b) => a + b, 0) / replicaCounts.length).toFixed(2)
    : "0.00";

  return (
    <main className="min-h-screen p-6 md:p-10">
      <h1 className="text-2xl font-semibold">DSprout Dashboard</h1>
      <p className="mt-1 text-sm text-gray-600">Satellite: {base}</p>

      <section className="mt-8">
        <h2 className="text-lg font-semibold">Workers</h2>
        {workersError ? (
          <p className="mt-2 text-sm text-red-700">Failed to load workers: {workersError}</p>
        ) : workers.length === 0 ? (
          <p className="mt-2 text-sm text-gray-600">No workers registered.</p>
        ) : (
          <div className="mt-3 overflow-auto border rounded">
            <table className="min-w-full text-sm">
              <thead>
                <tr className="bg-gray-100 text-left">
                  <th className="px-3 py-2">worker_id</th>
                  <th className="px-3 py-2">multiaddr</th>
                  <th className="px-3 py-2">last_seen</th>
                  <th className="px-3 py-2">health</th>
                </tr>
              </thead>
              <tbody>
                {workers.map((w) => {
                  const age = Math.max(0, newestLastSeen - Number(w.last_seen));
                  const healthy = age <= HEALTHY_MAX_AGE_MS;
                  return (
                    <tr key={w.worker_id} className="border-t">
                      <td className="px-3 py-2 font-mono">{w.worker_id}</td>
                      <td className="px-3 py-2 font-mono">{w.multiaddr}</td>
                      <td className="px-3 py-2">{fmtAge(age)}</td>
                      <td className="px-3 py-2">
                        <span className={healthy ? "text-green-700" : "text-red-700"}>
                          {healthy ? "healthy" : "stale"}
                        </span>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>

      <section className="mt-10">
        <h2 className="text-lg font-semibold">File Lookup</h2>
        <form className="mt-3 flex gap-2" method="get">
          <input
            name="file_id"
            defaultValue={fileId}
            placeholder="Enter file_id"
            className="w-full max-w-xl rounded border px-3 py-2 text-sm"
          />
          <button type="submit" className="rounded border px-4 py-2 text-sm font-medium">
            Query
          </button>
          <Link href="/" className="rounded border px-4 py-2 text-sm font-medium">
            Clear
          </Link>
        </form>

        {fileError ? <p className="mt-3 text-sm text-red-700">{fileError}</p> : null}

        {fileId ? (
          <div className="mt-4 space-y-3 text-sm">
            <div>
              <span className="font-semibold">file_id:</span> <span className="font-mono">{fileId}</span>
            </div>
            <div>
              <span className="font-semibold">segment count:</span>{" "}
              {manifest ? manifest.manifest.segments.length : "manifest not found"}
            </div>
            <div>
              <span className="font-semibold">shard records:</span> {locate?.shards.length || 0}
            </div>
            <div>
              <span className="font-semibold">unique shards:</span> {uniqueShards}
            </div>
            <div>
              <span className="font-semibold">replica counts:</span> min={minReplicas}, max={maxReplicas}, avg={avgReplicas}
            </div>
          </div>
        ) : (
          <p className="mt-3 text-sm text-gray-600">Enter a file_id to query manifest and shard locations.</p>
        )}
      </section>
    </main>
  );
}
