export type CubeState = { facelets: string };

export type SolveOk = {
  moves: string[];
  length: number;
  optimal: boolean;
  elapsedMs: number;
};

export type ScrambleOk = { moves: string[]; facelets: string };

export interface CubeApi {
  getSolved(): Promise<CubeState>;
  applyMoves(facelets: string, moves: string): Promise<CubeState>;
  validateFacelets(facelets: string): Promise<void>;
  scramble(n: number): Promise<ScrambleOk>;
  solveTwophase(facelets: string): Promise<SolveOk>;
}

type RequestBody =
  | { op: "get-solved" }
  | { op: "validate"; facelets: string }
  | { op: "apply-moves"; facelets: string; moves: string }
  | { op: "scramble"; n: number }
  | { op: "solve"; facelets: string };

async function post<T>(body: RequestBody): Promise<T> {
  const response = await fetch("/api/cube", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  const data = (await response.json()) as T & { message?: string };
  if (!response.ok) {
    throw new Error(data.message || `solver request failed (${response.status})`);
  }
  return data;
}

export function createHttpApi(): CubeApi {
  return {
    getSolved: () => post<CubeState>({ op: "get-solved" }),
    validateFacelets: async (facelets) => {
      await post({ op: "validate", facelets });
    },
    applyMoves: (facelets, moves) => post<CubeState>({ op: "apply-moves", facelets, moves }),
    scramble: (n) => post<ScrambleOk>({ op: "scramble", n }),
    solveTwophase: (facelets) => post<SolveOk>({ op: "solve", facelets }),
  };
}
