import * as THREE from "three";
import { BotView, ModuleKind, ModuleView } from "../lib/types";
import { degToRad, lerpAngle, setPos, toPos } from "./renderer";

const CELL_SIZE = 10;

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
};

interface ModuleVisual {
  root: THREE.Group;
  mat: THREE.MeshStandardMaterial;
  spinner?: THREE.Object3D;
  pulseMesh?: THREE.Object3D;
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
  baseAccentMat: THREE.MeshStandardMaterial,
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

    case ModuleKind.Thruster:
      root.add(new THREE.Mesh(GEOS.thruster, mat));
      return { root, mat };

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

  const baseColor = bot.overheated ? "#ef4444" : bot.color;

  bot.modules.forEach((m, i) => {
    const vis = entry.modules[i];
    if (!vis) return;

    vis.root.visible = m.hp_pct > 0;
    if (!vis.root.visible) return;

    const dmg = 1 - m.hp_pct;
    const lowHpFlicker =
      m.hp_pct < 0.36 && Math.sin(t * 25 + i * 3) > 0.4 ? 0.35 : 1.0;

    vis.mat.emissive
      .set(baseColor)
      .lerp(new THREE.Color("#ef4444"), dmg * 0.85);
    vis.mat.emissiveIntensity =
      (bot.overheated ? 2.5 : 0.6 + m.hp_pct * 1.4) * lowHpFlicker;

    if (vis.spinner) {
      vis.spinner.rotation.y = m.kind === ModuleKind.Core ? t * 1.8 : t * 4 + i;
    }
    if (vis.pulseMesh) {
      const s = 0.92 + Math.sin(t * 7 + i) * 0.1;
      vis.pulseMesh.scale.set(s, 1, s);
    }
  });
}

export function removeBotVisual(scene: THREE.Scene, entry: BotVisual) {
  scene.remove(entry.group);
  entry.mat.dispose();
  entry.accentMat.dispose();
  entry.modules.forEach((m) => m.mat.dispose());
}
