import * as THREE from "three";
import { BotView, ModuleKind, ModuleView } from "../lib/types";
import { degToRad, lerpAngle, setPos, toPos } from "./renderer";

const CELL_SIZE = 10;
const DMG_COLOR = new THREE.Color("#ef4444");
const MAX_SCAN_CONES = 4;
const CONE_SEGMENTS = 18;

const GEOS = {
  core: new THREE.OctahedronGeometry(5.2, 0),
  armor: new THREE.BoxGeometry(9.2, 7.5, 9.2),
  thruster: new THREE.CylinderGeometry(2.6, 4.0, 7, 8).rotateZ(Math.PI / 2),
  cannonBase: new THREE.BoxGeometry(6.5, 5, 6.5),
  cannonBarrel: new THREE.CylinderGeometry(1.1, 1.3, 9, 8).rotateZ(
    -Math.PI / 2,
  ),
  radarMast: new THREE.CylinderGeometry(1, 1.4, 6, 6),
  radarDish: new THREE.BoxGeometry(1.4, 4, 7),
  reactor: new THREE.CylinderGeometry(3.5, 3.5, 8.5, 10),
  flame: new THREE.ConeGeometry(2.5, 13, 8)
    .translate(0, 6.5, 0)
    .rotateZ(-Math.PI / 2),
  flameCore: new THREE.ConeGeometry(1.1, 8, 8)
    .translate(0, 4, 0)
    .rotateZ(-Math.PI / 2),
  baseRing: new THREE.RingGeometry(19, 21.2, 36).rotateX(-Math.PI / 2),
};

interface ModuleVisual {
  root: THREE.Group;
  mat: THREE.MeshStandardMaterial;
  spinner?: THREE.Object3D;
  pulseMesh?: THREE.Object3D;
  flame?: THREE.Mesh;
}

export interface BotVisual {
  group: THREE.Group;
  mat: THREE.MeshStandardMaterial;
  accentMat: THREE.MeshStandardMaterial;
  modules: ModuleVisual[];
  smoothedSpeed: number;
  targetPos: THREE.Vector3;
  flameMat: THREE.MeshBasicMaterial;
  flameCoreMat: THREE.MeshBasicMaterial;
  scanMat: THREE.MeshBasicMaterial;
  ringMat: THREE.MeshBasicMaterial;
  ring: THREE.Mesh;
  scans: ScanConeVisual[];
}

interface ScanConeVisual {
  mesh: THREE.Mesh;
  geo: THREE.BufferGeometry;
  lastFov: number;
  lastRange: number;
}

function createConeGeometry(): THREE.BufferGeometry {
  const geo = new THREE.BufferGeometry();
  const positions = new Float32Array((CONE_SEGMENTS + 2) * 3);
  const indices: number[] = [];
  for (let i = 1; i <= CONE_SEGMENTS; i++) {
    indices.push(0, i, i + 1);
  }
  geo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geo.setIndex(indices);
  return geo;
}

function updateConeGeometry(
  cone: ScanConeVisual,
  range: number,
  fovDeg: number,
) {
  const clampedFov = THREE.MathUtils.clamp(fovDeg, 2, 180);
  if (
    Math.abs(cone.lastFov - clampedFov) < 0.25 &&
    Math.abs(cone.lastRange - range) < 1
  ) {
    return;
  }
  cone.lastFov = clampedFov;
  cone.lastRange = range;

  const attr = cone.geo.getAttribute("position") as THREE.BufferAttribute;
  const half = THREE.MathUtils.degToRad(clampedFov * 0.5);
  attr.setXYZ(0, 0, 0, 0);

  for (let i = 0; i <= CONE_SEGMENTS; i++) {
    const a = -half + (i / CONE_SEGMENTS) * half * 2;
    attr.setXYZ(i + 1, Math.cos(a) * range, 0, Math.sin(a) * range);
  }
  attr.needsUpdate = true;
  cone.geo.computeBoundingSphere();
}

function buildModule(
  m: ModuleView,
  mainMat: THREE.MeshStandardMaterial,
  baseAccentMat: THREE.MeshStandardMaterial,
  flameMat: THREE.MeshBasicMaterial,
  flameCoreMat: THREE.MeshBasicMaterial,
): ModuleVisual {
  const root = new THREE.Group();
  root.position.set(m.gx * CELL_SIZE, 0, m.gy * CELL_SIZE);
  root.rotation.y = degToRad(m.dir);

  const mat = baseAccentMat.clone();

  switch (m.kind) {
    case ModuleKind.Core: {
      const core = new THREE.Mesh(GEOS.core, mat);
      root.add(core);
      return { root, mat, spinner: core };
    }

    case ModuleKind.Armor:
      root.add(new THREE.Mesh(GEOS.armor, mat));
      return { root, mat };

    case ModuleKind.Thruster: {
      const body = new THREE.Mesh(GEOS.thruster, mainMat);
      const flame = new THREE.Mesh(GEOS.flame, flameMat);
      const flameCore = new THREE.Mesh(GEOS.flameCore, flameCoreMat);
      flame.position.set(3.5, 0, 0);
      flameCore.position.set(3.5, 0, 0);
      flame.add(flameCore);
      root.add(body, flame);
      return { root, mat, flame };
    }

    case ModuleKind.Cannon: {
      const barrel = new THREE.Mesh(GEOS.cannonBarrel, mat);
      barrel.position.set(5, 1.5, 0);
      root.add(new THREE.Mesh(GEOS.cannonBase, mainMat), barrel);
      return { root, mat };
    }

    case ModuleKind.Radar: {
      const dish = new THREE.Mesh(GEOS.radarDish, mat);
      dish.position.y = 4.5;
      root.add(new THREE.Mesh(GEOS.radarMast, mainMat), dish);
      return { root, mat, spinner: dish };
    }

    case ModuleKind.Reactor: {
      const reactor = new THREE.Mesh(GEOS.reactor, mat);
      root.add(reactor);
      return { root, mat, pulseMesh: reactor };
    }

    default:
      return { root, mat };
  }
}

export function createBotVisual(scene: THREE.Scene, bot: BotView): BotVisual {
  const group = new THREE.Group();
  const mat = new THREE.MeshStandardMaterial({
    color: bot.color,
    emissive: bot.color,
    emissiveIntensity: 0.7,
    metalness: 0.5,
    roughness: 0.3,
  });
  const accentMat = new THREE.MeshStandardMaterial({
    color: 0xffffff,
    emissive: bot.color,
    emissiveIntensity: 2.2,
    metalness: 0.2,
    roughness: 0.1,
  });

  const flameMat = new THREE.MeshBasicMaterial({
    color: new THREE.Color(bot.color).multiplyScalar(3.2),
    transparent: true,
    opacity: 0.85,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
    toneMapped: false,
  });

  const flameCoreMat = new THREE.MeshBasicMaterial({
    color: new THREE.Color("#ffffff").multiplyScalar(4.0),
    transparent: true,
    opacity: 0.95,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
    toneMapped: false,
  });

  const scanMat = new THREE.MeshBasicMaterial({
    color: new THREE.Color(bot.color).multiplyScalar(1.8),
    transparent: true,
    opacity: 0.14,
    side: THREE.DoubleSide,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
    toneMapped: false,
  });

  const ringMat = new THREE.MeshBasicMaterial({
    color: new THREE.Color(bot.color).multiplyScalar(2.4),
    transparent: true,
    opacity: 0.35,
    side: THREE.DoubleSide,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
    toneMapped: false,
  });

  const ring = new THREE.Mesh(GEOS.baseRing, ringMat);
  ring.position.y = -12;
  group.add(ring);

  const scans: ScanConeVisual[] = Array.from({ length: MAX_SCAN_CONES }, () => {
    const geo = createConeGeometry();
    const mesh = new THREE.Mesh(geo, scanMat);
    mesh.position.y = -11;
    mesh.visible = false;
    group.add(mesh);
    return { mesh, geo, lastFov: 0, lastRange: 0 };
  });

  const modules = bot.modules.map((m) => {
    const vis = buildModule(m, mat, accentMat, flameMat, flameCoreMat);
    group.add(vis.root);
    return vis;
  });

  setPos(group.position, bot);
  group.rotation.y = degToRad(bot.angle);
  scene.add(group);

  return {
    group,
    mat,
    accentMat,
    flameMat,
    flameCoreMat,
    modules,
    smoothedSpeed: 0,
    targetPos: toPos(bot),
    scanMat,
    ringMat,
    ring,
    scans,
  };
}

export function updateBotVisual(entry: BotVisual, bot: BotView, t: number) {
  setPos(entry.targetPos, bot);
  const deltaDist = entry.group.position.distanceTo(entry.targetPos);
  entry.smoothedSpeed = THREE.MathUtils.lerp(
    entry.smoothedSpeed,
    Math.min(deltaDist / 4.5, 1.4),
    0.25,
  );

  entry.group.position.lerp(entry.targetPos, 0.35);
  entry.group.rotation.y = lerpAngle(
    entry.group.rotation.y,
    degToRad(bot.angle),
    0.35,
  );

  const col = bot.overheated ? "#ef4444" : bot.color;
  entry.flameMat.color
    .set(bot.overheated ? "#f97316" : bot.color)
    .multiplyScalar(3.2);
  entry.mat.color.set(col);
  entry.mat.emissive.set(col);
  entry.accentMat.emissive.set(col);
  entry.mat.emissiveIntensity = bot.overheated ? 2.5 : 0.7;

  const baseColor = bot.overheated ? "#ef4444" : bot.color;

  entry.ringMat.color.set(baseColor).multiplyScalar(2.4);
  entry.ringMat.opacity = bot.overheated ? 0.55 + Math.sin(t * 18) * 0.3 : 0.25;
  entry.ring.scale.setScalar(
    bot.overheated ? 1.15 + Math.sin(t * 14) * 0.08 : 1.0,
  );

  entry.scanMat.color.set(baseColor).multiplyScalar(1.8);
  const hasRadar = bot.modules.some(
    (m) => m.kind === ModuleKind.Radar && m.hp_pct > 0,
  );
  const scanRange = hasRadar ? 360 : 210;

  entry.scans.forEach((cone, i) => {
    const scan = bot.scans[i];
    if (!scan) {
      cone.mesh.visible = false;
      return;
    }
    const [offsetDeg, fovDeg] = scan;
    cone.mesh.visible = true;
    cone.mesh.rotation.y = degToRad(offsetDeg);
    updateConeGeometry(cone, scanRange, fovDeg);
  });

  bot.modules.forEach((m, i) => {
    const vis = entry.modules[i];
    if (!vis) return;

    vis.root.visible = m.hp_pct > 0;
    if (!vis.root.visible) return;

    const dmg = 1 - m.hp_pct;
    const lowHpFlicker =
      m.hp_pct < 0.36 && Math.sin(t * 25 + i * 3) > 0.4 ? 0.35 : 1.0;

    vis.mat.emissive.set(baseColor).lerp(DMG_COLOR, dmg * 0.85);
    vis.mat.emissiveIntensity =
      (bot.overheated ? 2.5 : 0.6 + m.hp_pct * 1.4) * lowHpFlicker;

    if (vis.spinner) {
      vis.spinner.rotation.y = m.kind === ModuleKind.Core ? t * 1.8 : t * 4 + i;
    }
    if (vis.pulseMesh) {
      const s = 0.92 + Math.sin(t * 7 + i) * 0.1;
      vis.pulseMesh.scale.set(s, 1, s);
    }
    if (vis.flame) {
      const thrust = THREE.MathUtils.clamp(
        0.45 + entry.smoothedSpeed * 1.1,
        0.45,
        2.0,
      );
      const flicker = 0.85 + Math.sin(t * 50 + i * 7) * 0.2;
      vis.flame.scale.set(
        thrust * flicker,
        0.85 + thrust * 0.15,
        0.85 + thrust * 0.15,
      );
    }
  });
}

export function removeBotVisual(scene: THREE.Scene, entry: BotVisual) {
  scene.remove(entry.group);
  entry.mat.dispose();
  entry.accentMat.dispose();
  entry.flameMat.dispose();
  entry.flameCoreMat.dispose();
  entry.modules.forEach((m) => m.mat.dispose());
  entry.scans.forEach((s) => s.geo.dispose());
}
