import type { CubeApi } from "./api";
import {
  applyMove,
  applyMoves,
  FACE_NAMES,
  inverseMove,
  parseMoves,
  SOLVED_FACELETS,
} from "./model";
import { CubeScene } from "./scene";

const TURN_MS = 320;
const HALF_MS = 420;

export function mountCubeApp(host: HTMLElement, api: CubeApi): () => void {
  host.classList.add("cube-app");
  host.replaceChildren();

  const header = el("header", "cube-header");
  const title = el("div", "cube-title");
  title.append(text("h1", "FaceTurn Cube Solver"), text("p", "Two-phase playback on a 3×3"));
  header.append(title);

  const stage = el("div", "cube-stage");
  const viewport = el("div", "cube-viewport");
  const panel = el("div", "cube-panel");
  stage.append(viewport, panel);

  const faceField = field("Facelets", "54 stickers, URFDLB");
  const faceInput = textarea(SOLVED_FACELETS, 3);
  const faceActions = row();
  const applyFace = button("Apply", "primary");
  const validateFace = button("Validate", "ghost");
  faceActions.append(applyFace, validateFace);
  faceField.append(faceInput, faceActions);

  const moveField = field("Moves", "Singmaster, applied to the cube you see");
  const moveInput = textarea("R U R' U'", 2);
  const applyMoveBtn = button("Apply moves", "ghost");
  moveField.append(moveInput, applyMoveBtn);

  const scrambleRow = row();
  const scrambleLabel = document.createElement("label");
  scrambleLabel.className = "cube-n";
  scrambleLabel.append("Shuffle");
  const scrambleN = document.createElement("input");
  scrambleN.type = "number";
  scrambleN.min = "0";
  scrambleN.max = "40";
  scrambleN.value = "25";
  scrambleN.setAttribute("aria-label", "Shuffle length");
  scrambleLabel.append(scrambleN);
  const shuffleBtn = button("Shuffle", "ghost");
  const resetBtn = button("Reset", "ghost");
  scrambleRow.append(scrambleLabel, shuffleBtn, resetBtn);

  const solveRow = row();
  const solveBtn = button("Solve", "primary");
  const optimalBtn = button("Optimal", "ghost");
  optimalBtn.disabled = true;
  optimalBtn.title = "Not wired in the UI yet.";
  solveRow.append(solveBtn, optimalBtn);

  const status = el("p", "cube-status");
  status.textContent = "Ready";

  const faces = el("div", "cube-faces");
  const faceHint = el("p", "cube-hint");
  faceHint.textContent = "Keys U R F D L B. Shift for prime. 2 then a face for a half turn.";
  for (const face of FACE_NAMES) {
    const line = el("div", "cube-face-line");
    for (const suffix of ["", "2", "'"] as const) {
      const token = `${face}${suffix}`;
      const control = button(token, "turn");
      control.addEventListener("click", () => manualTurn(token));
      line.append(control);
    }
    faces.append(line);
  }

  const playback = el("div", "cube-playback");
  const playBtn = button("Play", "primary");
  const pauseBtn = button("Pause", "ghost");
  const backBtn = button("Step back", "ghost");
  const fwdBtn = button("Step", "ghost");
  playback.append(backBtn, playBtn, pauseBtn, fwdBtn);

  const sliderLabel = document.createElement("label");
  sliderLabel.className = "cube-slider";
  const slider = document.createElement("input");
  slider.type = "range";
  slider.min = "0";
  slider.max = "0";
  slider.value = "0";
  slider.setAttribute("aria-label", "Solution step");
  const sliderRead = el("span", "cube-slider-read");
  sliderRead.textContent = "0 / 0";
  sliderLabel.append(slider, sliderRead);

  const scrambleNote = el("p", "cube-scramble");
  const moveList = el("ol", "cube-moves");

  panel.append(
    faceField,
    moveField,
    scrambleRow,
    faces,
    faceHint,
    solveRow,
    status,
    playback,
    sliderLabel,
    scrambleNote,
    moveList,
  );
  host.append(header, stage);

  const scene = new CubeScene(viewport);
  const observer = new ResizeObserver(() => scene.resize());
  observer.observe(viewport);

  let facelets = SOLVED_FACELETS;
  let solution: string[] | null = null;
  let startFacelets = facelets;
  let index = 0;
  let playing = false;
  let playbackBusy = false;
  let generation = 0;
  let solveEpoch = 0;
  let chain: Promise<void> = Promise.resolve();
  let armedDouble = false;

  const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const duration = (token: string) => {
    if (reduced()) return 0;
    return token.endsWith("2") ? HALF_MS : TURN_MS;
  };
  const current = (gen: number) => gen === generation;

  function note(message: string, tone: "ok" | "bad" | "muted" = "muted") {
    status.textContent = message;
    status.dataset.tone = tone;
  }

  function renderMoves() {
    moveList.replaceChildren();
    const moves = solution ?? [];
    if (moves.length === 0) {
      const empty = el("li", "cube-empty");
      empty.textContent = solution ? "Already solved" : "No solution yet";
      moveList.append(empty);
    } else {
      moves.forEach((token, i) => {
        const item = document.createElement("li");
        const chip = button(token, "chip");
        if (i === index) chip.classList.add("is-current");
        else if (i < index) chip.classList.add("is-done");
        chip.disabled = playbackBusy || playing;
        chip.addEventListener("click", () => jump(i));
        item.append(chip);
        moveList.append(item);
      });
    }
    const max = moves.length;
    const busy = playbackBusy || playing;
    slider.max = String(max);
    slider.value = String(Math.min(index, max));
    slider.disabled = solution === null || busy;
    sliderRead.textContent = `${index} / ${max}`;
    playBtn.disabled = busy || solution === null || index >= (solution?.length ?? 0);
    backBtn.disabled = busy || solution === null || index === 0;
    fwdBtn.disabled = busy || solution === null || index >= (solution?.length ?? 0);
  }

  function clearSolution() {
    solution = null;
    index = 0;
    startFacelets = facelets;
    renderMoves();
  }

  function snap(next: string) {
    playing = false;
    generation += 1;
    chain = Promise.resolve();
    facelets = next;
    scene.release();
    scene.setFacelets(facelets);
    faceInput.value = facelets;
  }

  function jump(i: number) {
    if (!solution) return;
    const next = Math.max(0, Math.min(solution.length, i));
    playing = false;
    generation += 1;
    chain = Promise.resolve();
    index = next;
    facelets = applyMoves(startFacelets, solution.slice(0, index));
    scene.release();
    scene.setFacelets(facelets);
    faceInput.value = facelets;
    renderMoves();
  }

  function enqueue(token: string): Promise<void> {
    solveEpoch += 1;
    const gen = generation;
    const run = chain.then(async () => {
      if (!current(gen)) return;
      const lived = await scene.animateTurn(token, duration(token), () => current(gen));
      if (!lived || !current(gen)) {
        if (current(gen)) scene.setFacelets(facelets);
        return;
      }
      facelets = applyMove(facelets, token);
      scene.setFacelets(facelets);
      faceInput.value = facelets;
    });
    chain = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
  }

  function manualTurn(token: string) {
    if (playing) return;
    if (solution) {
      clearSolution();
      note("Solution cleared");
    }
    void enqueue(token).catch((error: unknown) => note(messageOf(error), "bad"));
  }

  async function onApplyFacelets() {
    const text = faceInput.value.trim();
    try {
      await api.validateFacelets(text);
      snap(text);
      clearSolution();
      note("Facelets applied", "ok");
    } catch (error) {
      note(messageOf(error), "bad");
    }
  }

  async function onValidate() {
    try {
      await api.validateFacelets(faceInput.value.trim());
      note("Facelets are legal", "ok");
    } catch (error) {
      note(messageOf(error), "bad");
    }
  }

  async function onApplyMoves() {
    let moves: string[];
    try {
      moves = parseMoves(moveInput.value);
    } catch (error) {
      note(messageOf(error), "bad");
      return;
    }
    try {
      await api.applyMoves(facelets, moves.join(" "));
    } catch (error) {
      note(messageOf(error), "bad");
      return;
    }
    if (solution) clearSolution();
    note(moves.length === 0 ? "No moves" : `Applying ${moves.length}`);
    for (const token of moves) void enqueue(token);
  }

  async function onShuffle() {
    const n = Math.max(0, Math.min(40, Number(scrambleN.value) || 0));
    try {
      const result = await api.scramble(n);
      snap(result.facelets);
      clearSolution();
      scrambleNote.textContent = result.moves.length ? result.moves.join(" ") : "Identity";
      note(`Shuffled ${result.moves.length}`, "ok");
    } catch (error) {
      note(messageOf(error), "bad");
    }
  }

  async function onReset() {
    try {
      const solved = await api.getSolved();
      snap(solved.facelets);
    } catch {
      snap(SOLVED_FACELETS);
    }
    clearSolution();
    scrambleNote.textContent = "";
    note("Solved", "ok");
  }

  async function onSolve() {
    const requested = facelets;
    const epoch = ++solveEpoch;
    playing = false;
    generation += 1;
    const gen = generation;
    chain = Promise.resolve();
    scene.release();
    scene.setFacelets(requested);
    solveBtn.disabled = true;
    note("Solving…");
    try {
      const result = await api.solveTwophase(requested);
      if (epoch !== solveEpoch || !current(gen) || facelets !== requested) {
        if (current(gen)) note("Solve discarded; the cube changed");
        return;
      }
      startFacelets = requested;
      solution = result.moves;
      index = 0;
      note(`length ${result.length} · optimal false · ${result.elapsedMs} ms`, "ok");
      renderMoves();
    } catch (error) {
      if (current(gen)) note(messageOf(error), "bad");
    } finally {
      solveBtn.disabled = false;
    }
  }

  async function stepForward() {
    if (!solution || playing || playbackBusy || index >= solution.length) return;
    playbackBusy = true;
    renderMoves();
    const token = solution[index]!;
    const gen = generation;
    try {
      const lived = await scene.animateTurn(token, duration(token), () => current(gen));
      if (!lived || !current(gen) || !solution) {
        if (current(gen)) scene.setFacelets(facelets);
        return;
      }
      index += 1;
      facelets = applyMoves(startFacelets, solution.slice(0, index));
      scene.setFacelets(facelets);
      faceInput.value = facelets;
    } finally {
      playbackBusy = false;
      renderMoves();
    }
  }

  async function stepBack() {
    if (!solution || playing || playbackBusy || index === 0) return;
    playbackBusy = true;
    renderMoves();
    const token = inverseMove(solution[index - 1]!);
    const gen = generation;
    try {
      const lived = await scene.animateTurn(token, duration(token), () => current(gen));
      if (!lived || !current(gen) || !solution) {
        if (current(gen)) scene.setFacelets(facelets);
        return;
      }
      index -= 1;
      facelets = applyMoves(startFacelets, solution.slice(0, index));
      scene.setFacelets(facelets);
      faceInput.value = facelets;
    } finally {
      playbackBusy = false;
      renderMoves();
    }
  }

  async function onPlay() {
    if (!solution || playing || playbackBusy || index >= solution.length) return;
    playing = true;
    playbackBusy = true;
    renderMoves();
    const gen = generation;
    try {
      while (playing && solution && index < solution.length && current(gen)) {
        const token = solution[index]!;
        const lived = await scene.animateTurn(token, duration(token), () => current(gen) && playing);
        if (!lived || !playing || !current(gen) || !solution) {
          if (current(gen)) scene.setFacelets(facelets);
          break;
        }
        index += 1;
        facelets = applyMoves(startFacelets, solution.slice(0, index));
        scene.setFacelets(facelets);
        faceInput.value = facelets;
        renderMoves();
      }
    } finally {
      playing = false;
      playbackBusy = false;
      renderMoves();
    }
  }

  function onKey(event: KeyboardEvent) {
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "2") {
      armedDouble = true;
      return;
    }
    const code = event.code;
    const face = code.startsWith("Key") ? code.slice(3) : "";
    if (!FACE_NAMES.includes(face as (typeof FACE_NAMES)[number])) return;
    event.preventDefault();
    const suffix = event.shiftKey ? "'" : armedDouble ? "2" : "";
    armedDouble = false;
    manualTurn(`${face}${suffix}`);
  }

  applyFace.addEventListener("click", () => void onApplyFacelets());
  validateFace.addEventListener("click", () => void onValidate());
  applyMoveBtn.addEventListener("click", () => void onApplyMoves());
  shuffleBtn.addEventListener("click", () => void onShuffle());
  resetBtn.addEventListener("click", () => void onReset());
  solveBtn.addEventListener("click", () => void onSolve());
  playBtn.addEventListener("click", () => void onPlay());
  pauseBtn.addEventListener("click", () => {
    playing = false;
  });
  backBtn.addEventListener("click", () => void stepBack());
  fwdBtn.addEventListener("click", () => void stepForward());
  slider.addEventListener("input", () => jump(Number(slider.value)));
  window.addEventListener("keydown", onKey);

  renderMoves();
  const stopTables = watchTableEvents(note);
  void api.getSolved().then(
    (state) => {
      facelets = state.facelets;
      faceInput.value = facelets;
      scene.setFacelets(facelets);
    },
    () => note("Solver is not reachable yet", "bad"),
  );

  return () => {
    generation += 1;
    playing = false;
    stopTables();
    observer.disconnect();
    window.removeEventListener("keydown", onKey);
    scene.dispose();
    host.replaceChildren();
    host.classList.remove("cube-app");
  };
}

function messageOf(error: unknown): string {
  if (typeof error === "string" && error.trim()) return error;
  if (error instanceof Error && error.message) return error.message;
  if (error && typeof error === "object" && "message" in error) {
    const message = error.message;
    if (typeof message === "string" && message) return message;
  }
  return "Something went wrong";
}

function watchTableEvents(note: (message: string, tone?: "ok" | "bad" | "muted") => void): () => void {
  if (!("__TAURI_INTERNALS__" in window)) return () => undefined;
  const stop: Array<() => void> = [];
  let closed = false;
  void import("@tauri-apps/api/event").then(async ({ listen }) => {
    const unlistenProgress = await listen<string>("tables-progress", (event) => {
      note(event.payload, "muted");
    });
    const unlistenDone = await listen<string>("tables-done", () => undefined);
    if (closed) {
      unlistenProgress();
      unlistenDone();
      return;
    }
    stop.push(unlistenProgress, unlistenDone);
  });
  return () => {
    closed = true;
    for (const unlisten of stop) unlisten();
  };
}

function el(tag: string, className: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = className;
  return node;
}

function text(tag: string, value: string): HTMLElement {
  const node = document.createElement(tag);
  node.textContent = value;
  return node;
}

function field(label: string, hint: string): HTMLElement {
  const wrap = el("label", "cube-field");
  const name = el("span", "cube-label");
  name.textContent = label;
  const sub = el("span", "cube-field-hint");
  sub.textContent = hint;
  wrap.append(name, sub);
  return wrap;
}

function row(): HTMLElement {
  return el("div", "cube-row");
}

function button(label: string, kind: "primary" | "ghost" | "turn" | "chip"): HTMLButtonElement {
  const node = document.createElement("button");
  node.type = "button";
  node.className = `cube-btn cube-btn-${kind}`;
  node.textContent = label;
  return node;
}

function textarea(value: string, rows: number): HTMLTextAreaElement {
  const node = document.createElement("textarea");
  node.className = "cube-text";
  node.rows = rows;
  node.spellcheck = false;
  node.value = value;
  return node;
}
