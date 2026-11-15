import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import {
  BloomEffect,
  EffectComposer,
  EffectPass,
  RenderPass,
  ToneMappingEffect,
  ToneMappingMode,
} from "postprocessing";

export interface Vec2 {
  x: number;
  y: number;
}

export const setPos = (v: THREE.Vector3, p: THREE.Vector2Like, y = 14) =>
  v.set(p.x - 500, y, p.y - 500);

export const toPos = (p: THREE.Vector2Like, y = 14) =>
  new THREE.Vector3(p.x - 500, y, p.y - 500);

export const degToRad = (deg: number) => -(deg * Math.PI) / 180;

export function lerpAngle(current: number, target: number, t: number) {
  const diff = Math.atan2(
    Math.sin(target - current),
    Math.cos(target - current),
  );
  return current + diff * t;
}

export class MeshPool<T> {
  private items: THREE.Mesh[] = [];

  constructor(
    private scene: THREE.Scene,
    private geo: THREE.BufferGeometry,
    private mat: THREE.Material,
  ) {}

  sync(data: T[], update: (mesh: THREE.Mesh, item: T, i: number) => void) {
    while (this.items.length < data.length) {
      const m = new THREE.Mesh(this.geo, this.mat);
      this.scene.add(m);
      this.items.push(m);
    }
    this.items.forEach((m, i) => {
      m.visible = i < data.length;
      if (m.visible) update(m, data[i], i);
    });
  }
}

export function createArenaRenderer(container: HTMLElement) {
  const width = container.clientWidth || 800;
  const height = container.clientHeight || 700;

  // Scene
  const scene = new THREE.Scene();
  scene.background = new THREE.Color("#04040a");
  scene.fog = new THREE.FogExp2("#04040a", 0.00055);

  // Camera
  const camera = new THREE.PerspectiveCamera(50, width / height, 1, 5000);
  camera.position.set(0, 820, 720);

  // Renderer
  const renderer = new THREE.WebGLRenderer({
    powerPreference: "high-performance",
    antialias: false,
    stencil: false,
    depth: false,
  });
  renderer.setSize(width, height);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  container.appendChild(renderer.domElement);

  // Controls
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.maxPolarAngle = Math.PI / 2 - 0.05;

  // Lighting & Grid
  scene.add(new THREE.AmbientLight());
  const dirLight = new THREE.DirectionalLight(0xffffff, 1.5);
  dirLight.position.set(300, 600, 300);
  scene.add(dirLight);

  scene.add(new THREE.GridHelper(1000, 25, 0x1e293b, 0x0f172a));
  const border = new THREE.LineSegments(
    new THREE.EdgesGeometry(new THREE.BoxGeometry(1000, 24, 1000)),
    new THREE.LineBasicMaterial({ color: 0x38bdf8 }),
  );
  border.position.y = 12;
  scene.add(border);

  // Effects
  const composer = new EffectComposer(renderer, {
    frameBufferType: THREE.HalfFloatType,
    multisampling: 4,
  });
  composer.addPass(new RenderPass(scene, camera));
  composer.addPass(
    new EffectPass(
      camera,
      new BloomEffect({
        mipmapBlur: true,
        luminanceThreshold: 0.75,
        intensity: 1.15,
      }),
      new ToneMappingEffect({ mode: ToneMappingMode.ACES_FILMIC }),
    ),
  );

  const ro = new ResizeObserver(() => {
    const w = container.clientWidth;
    const h = container.clientHeight;
    if (!w || !h) return;

    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    composer.setSize(w, h);
  });
  ro.observe(container);

  return {
    scene,
    controls,
    render: () => {
      controls.update();
    },
    dispose: () => {
      ro.disconnect();
      controls.dispose();
      composer.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    },
  };
}
