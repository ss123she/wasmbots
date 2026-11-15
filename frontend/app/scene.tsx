"use client";
import { useEffect, useRef } from "react";
import * as THREE from "three";
import { ArenaSnapshot, BulletView, OrbView } from "./lib/types";
import { MeshPool, createArenaRenderer, setPos } from "./scene/renderer";
import {
  BotVisual,
  createBotVisual,
  removeBotVisual,
  updateBotVisual,
} from "./scene/bot-mesh";

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

    const bots = new Map<number, BotVisual>();

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

    const animate = () => {
      rafId = requestAnimationFrame(animate);
      const t = performance.now() * 0.001;
      const snap = latestRef.current;

      if (snap) {
        const active = new Set<number>();

        for (const bot of snap.bots) {
          active.add(bot.id);
          let entry = bots.get(bot.id);
          if (!entry) {
            entry = createBotVisual(scene, bot);
            bots.set(bot.id, entry);
          }
          updateBotVisual(entry, bot, t);
          if (followRef.current === bot.id) {
            controls.target.lerp(entry.group.position, 0.1);
          }
        }

        for (const [id, entry] of bots) {
          if (!active.has(id)) {
            removeBotVisual(scene, entry);
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
      bots.forEach((entry) => removeBotVisual(scene, entry));
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
