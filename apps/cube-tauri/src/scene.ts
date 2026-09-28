import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import {
  faceletIndex,
  MOVE_SPEC,
  SOLVED_FACELETS,
  turnRadians,
  type Axis,
} from "./model";

const STICKER: Record<string, number> = {
  U: 0xf4f4f2,
  R: 0xc44536,
  F: 0x2f9e57,
  D: 0xe3c84a,
  L: 0xe0702f,
  B: 0x2f62d6,
};

const GAP = 1.04;

type Cubie = {
  x: number;
  y: number;
  z: number;
  mesh: THREE.Group;
  stickers: { face: string; material: THREE.MeshStandardMaterial }[];
};

function easeInOut(t: number): number {
  return t < 0.5 ? 2 * t * t : 1 - ((-2 * t + 2) ** 2) / 2;
}

function axisVector(axis: Axis): THREE.Vector3 {
  if (axis === "x") return new THREE.Vector3(1, 0, 0);
  if (axis === "y") return new THREE.Vector3(0, 1, 0);
  return new THREE.Vector3(0, 0, 1);
}

export class CubeScene {
  readonly canvas: HTMLCanvasElement;
  private readonly renderer: THREE.WebGLRenderer;
  private readonly scene = new THREE.Scene();
  private readonly camera: THREE.PerspectiveCamera;
  private readonly controls: OrbitControls;
  private readonly pivot = new THREE.Group();
  private readonly cubies: Cubie[] = [];
  private frame = 0;
  private disposed = false;

  constructor(host: HTMLElement) {
    this.canvas = document.createElement("canvas");
    this.canvas.className = "cube-canvas";
    host.append(this.canvas);

    this.renderer = new THREE.WebGLRenderer({
      canvas: this.canvas,
      antialias: true,
      alpha: false,
    });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.setClearColor(0x121214, 1);
    this.renderer.shadowMap.enabled = true;

    this.camera = new THREE.PerspectiveCamera(32, 1, 0.1, 50);
    this.camera.position.set(3.55, 2.85, 4.35);

    this.scene.add(new THREE.AmbientLight(0xf2f0ec, 0.55));
    const key = new THREE.DirectionalLight(0xfff8ee, 1.35);
    key.position.set(4.5, 7.5, 5);
    key.castShadow = true;
    key.shadow.mapSize.set(1024, 1024);
    this.scene.add(key);
    const fill = new THREE.DirectionalLight(0xc5d0e0, 0.35);
    fill.position.set(-5, 2, -3);
    this.scene.add(fill);

    this.scene.add(this.pivot);
    this.buildCubies();

    const floor = new THREE.Mesh(
      new THREE.CircleGeometry(3.4, 48),
      new THREE.MeshStandardMaterial({ color: 0x1a1b1f, roughness: 1 }),
    );
    floor.rotation.x = -Math.PI / 2;
    floor.position.y = -1.72;
    floor.receiveShadow = true;
    this.scene.add(floor);

    this.controls = new OrbitControls(this.camera, this.canvas);
    this.controls.enablePan = false;
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.autoRotate = false;
    this.controls.minDistance = 4.2;
    this.controls.maxDistance = 11;
    this.controls.target.set(0, 0, 0);

    this.setFacelets(SOLVED_FACELETS);
    this.resize();
    const loop = () => {
      if (this.disposed) return;
      this.frame = requestAnimationFrame(loop);
      this.controls.update();
      this.renderer.render(this.scene, this.camera);
    };
    loop();
  }

  resize(): void {
    const parent = this.canvas.parentElement;
    if (!parent) return;
    const width = Math.max(1, parent.clientWidth);
    const height = Math.max(1, parent.clientHeight);
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(width, height, false);
  }

  setFacelets(facelets: string): void {
    this.release();
    for (const cubie of this.cubies) {
      cubie.mesh.position.set(cubie.x * GAP, cubie.y * GAP, cubie.z * GAP);
      cubie.mesh.quaternion.identity();
      for (const sticker of cubie.stickers) {
        const index = faceletIndex(cubie.x, cubie.y, cubie.z, sticker.face);
        sticker.material.color.setHex(STICKER[facelets[index] ?? ""] ?? 0x333333);
      }
    }
  }

  /** Rotate the layer, then the caller snaps colors from the new facelets. */
  animateTurn(token: string, durationMs: number, stillCurrent: () => boolean): Promise<boolean> {
    const spec = MOVE_SPEC[token];
    if (!spec) return Promise.resolve(false);
    const moving = this.cubies.filter((cubie) => {
      const layer = spec.axis === "x" ? cubie.x : spec.axis === "y" ? cubie.y : cubie.z;
      return layer === spec.layer;
    });
    for (const cubie of moving) this.pivot.attach(cubie.mesh);
    const axis = axisVector(spec.axis);
    const target = turnRadians(token);
    if (durationMs <= 0) {
      if (!stillCurrent()) {
        this.release();
        return Promise.resolve(false);
      }
      return Promise.resolve(true);
    }
    const started = performance.now();
    return new Promise((resolve) => {
      const step = (now: number) => {
        if (!stillCurrent()) {
          this.release();
          resolve(false);
          return;
        }
        const t = Math.min(1, (now - started) / durationMs);
        this.pivot.quaternion.setFromAxisAngle(axis, target * easeInOut(t));
        if (t < 1) {
          requestAnimationFrame(step);
          return;
        }
        resolve(true);
      };
      requestAnimationFrame(step);
    });
  }

  release(): void {
    for (const cubie of this.cubies) {
      if (cubie.mesh.parent !== this.scene) this.scene.attach(cubie.mesh);
    }
    this.pivot.quaternion.identity();
  }

  dispose(): void {
    this.disposed = true;
    cancelAnimationFrame(this.frame);
    this.controls.dispose();
    this.renderer.dispose();
    this.canvas.remove();
  }

  private buildCubies(): void {
    const box = new THREE.BoxGeometry(0.94, 0.94, 0.94);
    const plastic = new THREE.MeshStandardMaterial({
      color: 0x141416,
      roughness: 0.72,
      metalness: 0.04,
    });
    const stickerGeo = new THREE.PlaneGeometry(0.78, 0.78);
    for (const x of [-1, 0, 1]) {
      for (const y of [-1, 0, 1]) {
        for (const z of [-1, 0, 1]) {
          if (x === 0 && y === 0 && z === 0) continue;
          const mesh = new THREE.Group();
          const body = new THREE.Mesh(box, plastic);
          body.castShadow = true;
          body.receiveShadow = true;
          mesh.add(body);
          const stickers: Cubie["stickers"] = [];
          const add = (face: string, place: (mesh: THREE.Mesh) => void) => {
            const material = new THREE.MeshStandardMaterial({
              color: 0xffffff,
              roughness: 0.42,
              metalness: 0.02,
            });
            const sticker = new THREE.Mesh(stickerGeo, material);
            place(sticker);
            mesh.add(sticker);
            stickers.push({ face, material });
          };
          const lift = 0.478;
          if (y === 1) {
            add("U", (m) => {
              m.position.y = lift;
              m.rotation.x = -Math.PI / 2;
            });
          }
          if (y === -1) {
            add("D", (m) => {
              m.position.y = -lift;
              m.rotation.x = Math.PI / 2;
            });
          }
          if (x === 1) {
            add("R", (m) => {
              m.position.x = lift;
              m.rotation.y = Math.PI / 2;
            });
          }
          if (x === -1) {
            add("L", (m) => {
              m.position.x = -lift;
              m.rotation.y = -Math.PI / 2;
            });
          }
          if (z === 1) add("F", (m) => { m.position.z = lift; });
          if (z === -1) {
            add("B", (m) => {
              m.position.z = -lift;
              m.rotation.y = Math.PI;
            });
          }
          this.scene.add(mesh);
          this.cubies.push({ x, y, z, mesh, stickers });
        }
      }
    }
  }
}
