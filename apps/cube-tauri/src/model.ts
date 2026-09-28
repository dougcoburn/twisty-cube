/** Facelet geometry matching `facelets.rs`: URFDLB, 3×3, face toward you. */

export const SOLVED_FACELETS =
  "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";

export type Axis = "x" | "y" | "z";

type Spec = {
  axis: Axis;
  layer: -1 | 1;
  /** +1 is a right-handed quarter turn around `axis`. */
  quarter: 1 | -1;
  times: 1 | 2;
};

/** Clockwise face turns, verified against the solver's move cubes. */
export const MOVE_SPEC: Record<string, Spec> = {
  U: { axis: "y", layer: 1, quarter: 1, times: 1 },
  U2: { axis: "y", layer: 1, quarter: 1, times: 2 },
  "U'": { axis: "y", layer: 1, quarter: -1, times: 1 },
  R: { axis: "x", layer: 1, quarter: -1, times: 1 },
  R2: { axis: "x", layer: 1, quarter: -1, times: 2 },
  "R'": { axis: "x", layer: 1, quarter: 1, times: 1 },
  F: { axis: "z", layer: 1, quarter: -1, times: 1 },
  F2: { axis: "z", layer: 1, quarter: -1, times: 2 },
  "F'": { axis: "z", layer: 1, quarter: 1, times: 1 },
  D: { axis: "y", layer: -1, quarter: -1, times: 1 },
  D2: { axis: "y", layer: -1, quarter: -1, times: 2 },
  "D'": { axis: "y", layer: -1, quarter: 1, times: 1 },
  L: { axis: "x", layer: -1, quarter: 1, times: 1 },
  L2: { axis: "x", layer: -1, quarter: 1, times: 2 },
  "L'": { axis: "x", layer: -1, quarter: -1, times: 1 },
  B: { axis: "z", layer: -1, quarter: 1, times: 1 },
  B2: { axis: "z", layer: -1, quarter: 1, times: 2 },
  "B'": { axis: "z", layer: -1, quarter: -1, times: 1 },
};

export const FACE_NAMES = ["U", "R", "F", "D", "L", "B"] as const;

type Triple = [number, number, number];

export function rot90(axis: Axis, quarter: 1 | -1, p: Triple): Triple {
  const [x, y, z] = p;
  if (axis === "y") return quarter === 1 ? [-z, y, x] : [z, y, -x];
  if (axis === "x") return quarter === 1 ? [x, -z, y] : [x, z, -y];
  return quarter === 1 ? [-y, x, z] : [y, -x, z];
}

/** Index in the 54-char string for the sticker on `face` of the cubie at x,y,z in {-1,0,1}. */
export function faceletIndex(x: number, y: number, z: number, face: string): number {
  switch (face) {
    case "U":
      return (z + 1) * 3 + (x + 1);
    case "R":
      return 9 + (1 - y) * 3 + (1 - z);
    case "F":
      return 18 + (1 - y) * 3 + (x + 1);
    case "D":
      return 27 + (1 - z) * 3 + (x + 1);
    case "L":
      return 36 + (1 - y) * 3 + (z + 1);
    case "B":
      return 45 + (1 - y) * 3 + (1 - x);
    default:
      throw new Error(`unknown face ${face}`);
  }
}

function faceOfNormal(n: Triple): string {
  const [x, y, z] = n.map((v) => Math.round(v)) as Triple;
  if (y === 1) return "U";
  if (y === -1) return "D";
  if (x === 1) return "R";
  if (x === -1) return "L";
  if (z === 1) return "F";
  if (z === -1) return "B";
  throw new Error(`normal ${n.join(",")}`);
}

function stickers(x: number, y: number, z: number): { face: string; n: Triple }[] {
  const out: { face: string; n: Triple }[] = [];
  if (y === 1) out.push({ face: "U", n: [0, 1, 0] });
  if (y === -1) out.push({ face: "D", n: [0, -1, 0] });
  if (x === 1) out.push({ face: "R", n: [1, 0, 0] });
  if (x === -1) out.push({ face: "L", n: [-1, 0, 0] });
  if (z === 1) out.push({ face: "F", n: [0, 0, 1] });
  if (z === -1) out.push({ face: "B", n: [0, 0, -1] });
  return out;
}

export function applyMove(facelets: string, token: string): string {
  const spec = MOVE_SPEC[token];
  if (!spec) throw new Error(`unknown move ${token}`);
  const next = facelets.split("");
  for (const x of [-1, 0, 1]) {
    for (const y of [-1, 0, 1]) {
      for (const z of [-1, 0, 1]) {
        if (x === 0 && y === 0 && z === 0) continue;
        const layer = spec.axis === "x" ? x : spec.axis === "y" ? y : z;
        if (layer !== spec.layer) continue;
        for (const sticker of stickers(x, y, z)) {
          let p: Triple = [x, y, z];
          let n = sticker.n;
          for (let i = 0; i < spec.times; i += 1) {
            p = rot90(spec.axis, spec.quarter, p);
            n = rot90(spec.axis, spec.quarter, n);
          }
          const from = faceletIndex(x, y, z, sticker.face);
          const to = faceletIndex(p[0], p[1], p[2], faceOfNormal(n));
          next[to] = facelets[from] ?? "?";
        }
      }
    }
  }
  return next.join("");
}

export function parseMoves(text: string): string[] {
  const trimmed = text.trim();
  if (!trimmed) return [];
  const moves: string[] = [];
  for (const token of trimmed.split(/\s+/)) {
    if (!MOVE_SPEC[token]) throw new Error(`unknown move ${token}`);
    moves.push(token);
  }
  return moves;
}

export function applyMoves(facelets: string, moves: readonly string[]): string {
  return moves.reduce((state, token) => applyMove(state, token), facelets);
}

export function inverseMove(token: string): string {
  if (token.endsWith("2")) return token;
  if (token.endsWith("'")) return token.slice(0, -1);
  return `${token}'`;
}

export function turnRadians(token: string): number {
  const spec = MOVE_SPEC[token];
  if (!spec) return 0;
  // +Y in Three.js is the opposite of rot90's clockwise-from-above quarter.
  const handed = spec.axis === "y" ? -1 : 1;
  return handed * spec.quarter * spec.times * (Math.PI / 2);
}
