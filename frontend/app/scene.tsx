"use client";
import { useEffect, useRef } from "react";
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

export interface BotView {
  id: number;
  name: string;
  color: string;
  x: number;
  y: number;
  angle: number;
  hp: number;
  kills: number;
  cpu_pct: number;
  overheated: boolean;
  scans: [number, number][];
}

export interface BulletView {
  x: number;
  y: number;
}

export interface OrbView {
  x: number;
  y: number;
  kind: string;
}

export interface ArenaSnapshot {
  tick: number;
  bots: BotView[];
  bullets: BulletView[];
  orbs: OrbView[];
  events: string[];
}

interface ArenaSceneProps {
  snapshot: ArenaSnapshot | null;
  followBotId?: number | null;
}

function lerpAngle(current: number, target: number, t: number) {
  const diff = Math.atan2(
    Math.sin(target - current),
    Math.cos(target - current),
  );
  return current + diff * t;
}

export default function ArenaScene({
  snapshot,
  followBotId = null,
}: ArenaSceneProps) {
  const mountRef = useRef<HTMLDivElement>(null);
  const latestRef = useRef<ArenaSnapshot | null>(snapshot);
  const followRef = useRef<number | null>(followBotId);

  useEffect(() => {
    latestRef.current = snapshot;
    followRef.current = followBotId;
  }, [snapshot, followBotId]);

  useEffect(() => {
    const container = mountRef.current;
    if (!container) return;

    const width = container.clientWidth || 800;
    const height = container.clientHeight || 700;

    // Scene & Lighting
    const scene = new THREE.Scene();
    scene.background = new THREE.Color("#04040a");
    scene.fog = new THREE.FogExp2("#04040a", 0.00055);

    const camera = new THREE.PerspectiveCamera(50, width / height, 1, 5000);
    camera.position.set(0, 820, 720);

    const renderer = new THREE.WebGLRenderer({
      powerPreference: "high-performance",
      antialias: false,
      stencil: false,
      depth: false,
    });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    container.appendChild(renderer.domElement);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.maxPolarAngle = Math.PI / 2 - 0.05;

    scene.add(new THREE.AmbientLight(0xffffff, 0.5));
    const dirLight = new THREE.DirectionalLight(0xffffff, 1.5);
    dirLight.position.set(300, 600, 300);
    scene.add(dirLight);

    // Floor & Grid
    const grid = new THREE.GridHelper(1000, 25, 0x1e293b, 0x0f172a);
    scene.add(grid);

    const borderGeo = new THREE.EdgesGeometry(
      new THREE.BoxGeometry(1000, 24, 1000),
    );
    const borderMat = new THREE.LineBasicMaterial({ color: 0x38bdf8 });
    const border = new THREE.LineSegments(borderGeo, borderMat);
    border.position.y = 12;
    scene.add(border);

    // Post-processing effects
    const composer = new EffectComposer(renderer, {
      frameBufferType: THREE.HalfFloatType,
      multisampling: 4,
    });
    composer.addPass(new RenderPass(scene, camera));

    const bloomEffect = new BloomEffect({
      mipmapBlur: true,
      luminanceThreshold: 0.75,
      intensity: 1.15,
    });
    const toneMappingEffect = new ToneMappingEffect({
      mode: ToneMappingMode.ACES_FILMIC,
    });

    composer.addPass(new EffectPass(camera, bloomEffect, toneMappingEffect));

    // Geometry and object pools
    const shipGeo = new THREE.ConeGeometry(14, 34, 4);
    shipGeo.rotateZ(-Math.PI / 2);

    const botMeshes = new Map<
      number,
      { group: THREE.Group; mat: THREE.MeshStandardMaterial }
    >();

    const bulletGeo = new THREE.SphereGeometry(5, 8, 8);
    const bulletMat = new THREE.MeshBasicMaterial({
      color: new THREE.Color(0xff2a6d).multiplyScalar(2.5),
      toneMapped: false,
    });
    const bulletPool: THREE.Mesh[] = [];

    const orbGeo = new THREE.OctahedronGeometry(13, 0);
    const healMat = new THREE.MeshStandardMaterial({
      color: 0x4ade80,
      emissive: 0x4ade80,
      emissiveIntensity: 2.2,
    });
    const overclockMat = new THREE.MeshStandardMaterial({
      color: 0xfacc15,
      emissive: 0xfacc15,
      emissiveIntensity: 2.5,
    });
    const orbPool: THREE.Mesh[] = [];

    const getFromPool = (
      pool: THREE.Mesh[],
      idx: number,
      geo: THREE.BufferGeometry,
      mat: THREE.Material,
    ) => {
      while (pool.length <= idx) {
        const m = new THREE.Mesh(geo, mat);
        m.visible = false;
        scene.add(m);
        pool.push(m);
      }
      return pool[idx];
    };

    // Render cycle
    let rafId = 0;
    const clock = new THREE.Timer();

    const animate = () => {
      rafId = requestAnimationFrame(animate);
      const t = clock.getElapsed();
      const snap = latestRef.current;

      if (snap) {
        const activeIds = new Set<number>();

        // Bots
        for (const bot of snap.bots) {
          activeIds.add(bot.id);
          let entry = botMeshes.get(bot.id);

          if (!entry) {
            const group = new THREE.Group();
            const mat = new THREE.MeshStandardMaterial({
              color: bot.color,
              emissive: bot.color,
              emissiveIntensity: 1.6,
              metalness: 0.4,
              roughness: 0.2,
            });
            const body = new THREE.Mesh(shipGeo, mat);
            group.add(body);

            // TODO: Add something interesting here.

            group.position.set(bot.x - 500, 14, bot.y - 500);
            group.rotation.y = -(bot.angle * Math.PI) / 180;
            scene.add(group);
            entry = { group, mat };
            botMeshes.set(bot.id, entry);
          }

          entry.group.position.lerp(
            new THREE.Vector3(bot.x - 500, 14, bot.y - 500),
            0.35,
          );
          const targetRot = -(bot.angle * Math.PI) / 180;
          entry.group.rotation.y = lerpAngle(
            entry.group.rotation.y,
            targetRot,
            0.35,
          );

          if (bot.overheated) {
            entry.mat.color.set("#ef4444");
            entry.mat.emissive.set("#ef4444");
            entry.mat.emissiveIntensity = 3.0;
          } else {
            entry.mat.color.set(bot.color);
            entry.mat.emissive.set(bot.color);
            entry.mat.emissiveIntensity = 1.6;
          }

          if (followRef.current === bot.id) {
            controls.target.lerp(entry.group.position, 0.1);
          }
        }

        for (const [id, entry] of botMeshes.entries()) {
          if (!activeIds.has(id)) {
            scene.remove(entry.group);
            entry.mat.dispose();
            botMeshes.delete(id);
          }
        }

        // Bullets
        snap.bullets.forEach((b, i) => {
          const mesh = getFromPool(bulletPool, i, bulletGeo, bulletMat);
          mesh.visible = true;
          mesh.position.set(b.x - 500, 14, b.y - 500);
        });
        for (let i = snap.bullets.length; i < bulletPool.length; i++) {
          bulletPool[i].visible = false;
        }

        // Orbs
        snap.orbs.forEach((o, i) => {
          const mat = o.kind === "overclock" ? overclockMat : healMat;
          const mesh = getFromPool(orbPool, i, orbGeo, mat);
          mesh.material = mat;
          mesh.visible = true;
          mesh.position.set(o.x - 500, 16 + Math.sin(t * 4 + i) * 4, o.y - 500);
          mesh.rotation.y = t * 2 + i;
        });
        for (let i = snap.orbs.length; i < orbPool.length; i++) {
          orbPool[i].visible = false;
        }
      }

      controls.update();
      composer.render();
    };

    animate();

    // Adaptive resize
    const ro = new ResizeObserver(() => {
      const w = container.clientWidth;
      const h = container.clientHeight;
      if (!w || !h) return;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      composer.setSize(w, h);
    });
    ro.observe(container);

    return () => {
      cancelAnimationFrame(rafId);
      ro.disconnect();
      controls.dispose();
      composer.dispose();
      renderer.dispose();
      if (container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement);
      }
    };
  }, []);

  return (
    <div
      ref={mountRef}
      className="w-full h-full min-h-162.5 rounded-lg overflow-hidden border border-zinc-800"
    />
  );
}
