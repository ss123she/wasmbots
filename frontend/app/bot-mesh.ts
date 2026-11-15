import * as THREE from "three";
import { BotView, ModuleKind, ModuleView } from "./types";
import { degToRad, lerpAngle, setPos, toPos } from "./renderer";

const CELL_SIZE = 10;

const GEOS = {
  core: new THREE.OctahedronGeometry(5.2, 0),
  armor: new THREE.BoxGeometry(9.2, 7.5, 9.2),
  thruster: new THREE.CylinderGeometry(2.6, 4.0, 7, 8).rotateZ(Math.PI / 2),
  exhaust: new THREE.ConeGeometry(2.3, 7, 6).rotateZ(-Math.PI / 2),
  cannonBase: new THREE.BoxGeometry(6.5, 5, 6.5),
  cannonBarrel: new THREE.CylinderGeometry(1.1, 1.3, 9, 8).rotateZ(
    -Math.PI / 2,
  ),
  radarMast: new THREE.CylinderGeometry(1, 1.4, 6, 6),
  radarDish: new THREE.BoxGeometry(1.4, 4, 7),
  reactor: new THREE.CylinderGeometry(3.5, 3.5, 8.5, 10),
};

const exhaustMat = new THREE.MeshBasicMaterial({
  color: new THREE.Color(0x38bdf8).multiplyScalar(2.2),
  toneMapped: false,
});

interface ModuleVisual {
  root: THREE.Group;
  spinner?: THREE.Object3D;
  exhaust?: THREE.Object3D;
}

export interface BotVisual {
  group: THREE.Group;
  mat: THREE.MeshStandardMaterial;
  accentMat: THREE.MeshStandardMaterial;
  modules: ModuleVisual[];
}

function buildModule(
  m: ModuleView,
  mainMat: THREE.MeshStandardMaterial,
  accentMat: THREE.MeshStandardMaterial,
): ModuleVisual {
  const root = new THREE.Group();
  root.position.set(m.gx * CELL_SIZE, 0, m.gy * CELL_SIZE);
  root.rotation.y = degToRad(m.dir);

  switch (m.kind) {
    case ModuleKind.Core:
      root.add(new THREE.Mesh(GEOS.core, accentMat));
      return { root };

    case ModuleKind.Armor:
      root.add(new THREE.Mesh(GEOS.armor, accentMat));
      return { root };

    case ModuleKind.Thruster:
      root.add(new THREE.Mesh(GEOS.thruster, accentMat));
      return { root };

    case ModuleKind.Cannon:
      const barrel = new THREE.Mesh(GEOS.cannonBarrel, accentMat);
      barrel.position.set(5, 1.5, 0);
      root.add(new THREE.Mesh(GEOS.cannonBase, mainMat), barrel);
      return { root };

    case ModuleKind.Radar:
      const dish = new THREE.Mesh(GEOS.radarDish, accentMat);
      dish.position.y = 4.5;
      root.add(new THREE.Mesh(GEOS.radarMast, mainMat), dish);
      return { root, spinner: dish };

    case ModuleKind.Reactor:
      root.add(new THREE.Mesh(GEOS.reactor, accentMat));
      return { root };

    default:
      return { root };
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

  const modules = bot.modules.map((m) => {
    const vis = buildModule(m, mat, accentMat);
    group.add(vis.root);
    return vis;
  });

  setPos(group.position, bot);
  group.rotation.y = degToRad(bot.angle);
  scene.add(group);

  return { group, mat, accentMat, modules };
}

export function updateBotVisual(entry: BotVisual, bot: BotView, t: number) {
  entry.group.position.lerp(toPos(bot), 0.35);
  entry.group.rotation.y = lerpAngle(
    entry.group.rotation.y,
    degToRad(bot.angle),
    0.35,
  );

  const col = bot.overheated ? "#ef4444" : bot.color;
  entry.mat.color.set(col);
  entry.mat.emissive.set(col);
  entry.accentMat.emissive.set(col);
  entry.mat.emissiveIntensity = bot.overheated ? 2.5 : 0.7;

  bot.modules.forEach((m, i) => {
    const vis = entry.modules[i];
    if (!vis) return;
    vis.root.visible = m.hp_pct > 0;
    if (vis.spinner) vis.spinner.rotation.y = t * 4 + i;
    if (vis.exhaust)
      vis.exhaust.scale.set(0.75 + Math.sin(t * 24 + i) * 0.35, 1, 1);
  });
}

export function removeBotVisual(scene: THREE.Scene, entry: BotVisual) {
  scene.remove(entry.group);
  entry.mat.dispose();
  entry.accentMat.dispose();
}
