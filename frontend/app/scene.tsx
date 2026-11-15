"use client";
import { useEffect, useRef } from "react";
import * as THREE from "three";
import { ArenaSnapshot, BulletView, OrbView } from "./types";
import {
  MeshPool,
  createArenaRenderer,
  degToRad,
  lerpAngle,
  setPos,
  toPos,
} from "./renderer";

interface ArenaSceneProps {
  snapshot: ArenaSnapshot | null;
  followBotId?: number | null;
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

    const arena = createArenaRenderer(container);
    const { scene, controls } = arena;

    const shipGeo = new THREE.ConeGeometry(14, 34, 4).rotateZ(-Math.PI / 2);
    const bots = new Map<
      number,
      { mesh: THREE.Mesh; mat: THREE.MeshStandardMaterial }
    >();

    const bullets = new MeshPool<BulletView>(
      scene,
      new THREE.SphereGeometry(5, 8, 8),
      new THREE.MeshBasicMaterial({
        color: new THREE.Color(0xff2a6d).multiplyScalar(2.5),
        toneMapped: false,
      }),
    );

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
    const orbs = new MeshPool<OrbView>(
      scene,
      new THREE.OctahedronGeometry(13, 0),
      healMat,
    );

    let rafId = 0;
    const clock = new THREE.Timer();

    const animate = () => {
      rafId = requestAnimationFrame(animate);
      const t = clock.getElapsed();
      const snap = latestRef.current;

      if (snap) {
        const active = new Set<number>();

        for (const bot of snap.bots) {
          active.add(bot.id);
          let entry = bots.get(bot.id);

          if (!entry) {
            const mat = new THREE.MeshStandardMaterial({
              color: bot.color,
              emissive: bot.color,
              emissiveIntensity: 1.6,
              metalness: 0.4,
              roughness: 0.2,
            });
            const mesh = new THREE.Mesh(shipGeo, mat);
            setPos(mesh.position, bot);
            mesh.rotation.y = degToRad(bot.angle);
            scene.add(mesh);
            entry = { mesh, mat };
            bots.set(bot.id, entry);
          }

          entry.mesh.position.lerp(toPos(bot), 0.35);
          entry.mesh.rotation.y = lerpAngle(
            entry.mesh.rotation.y,
            degToRad(bot.angle),
            0.35,
          );

          const col = bot.overheated ? "#ef4444" : bot.color;
          entry.mat.color.set(col);
          entry.mat.emissive.set(col);
          entry.mat.emissiveIntensity = bot.overheated ? 3.0 : 1.6;

          if (followRef.current === bot.id) {
            controls.target.lerp(entry.mesh.position, 0.1);
          }
        }

        for (const [id, entry] of bots) {
          if (!active.has(id)) {
            scene.remove(entry.mesh);
            entry.mat.dispose();
            bots.delete(id);
          }
        }

        bullets.sync(snap.bullets, (m, b) => setPos(m.position, b));
        orbs.sync(snap.orbs, (m, o, i) => {
          m.material = o.kind === "overclock" ? overclockMat : healMat;
          setPos(m.position, o, 16 + Math.sin(t * 4 + i) * 4);
          m.rotation.y = t * 2 + i;
        });
      }

      arena.render();
    };

    animate();

    return () => {
      cancelAnimationFrame(rafId);
      arena.dispose();
    };
  }, []);

  return (
    <div
      ref={mountRef}
      className="w-full h-full min-h-162.5 rounded-lg overflow-hidden border border-zinc-800"
    />
  );
}
