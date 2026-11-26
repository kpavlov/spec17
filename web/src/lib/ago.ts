// (WEB-66) how long ago, coarsely: the header ticks twice a minute
export const ago = (ms: number) => (ms < 60_000 ? "just now" : ms < 3_600_000 ? `${Math.floor(ms / 60_000)}m ago` : `${Math.floor(ms / 3_600_000)}h ago`);
