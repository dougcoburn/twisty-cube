import type { CubeApi, CubeState, ScrambleOk, SolveOk } from "./api";

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: call } = await import("@tauri-apps/api/core");
  return call<T>(command, args);
}

export function createTauriApi(): CubeApi {
  return {
    getSolved: () => invoke<CubeState>("get_solved"),
    validateFacelets: (facelets) => invoke<void>("validate_facelets", { facelets }),
    applyMoves: (facelets, moves) => invoke<CubeState>("apply_moves", { facelets, moves }),
    scramble: (n) => invoke<ScrambleOk>("scramble", { n }),
    solveTwophase: (facelets) => invoke<SolveOk>("solve_twophase", { facelets }),
  };
}
