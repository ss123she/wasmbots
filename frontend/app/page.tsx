export default async function Home() {
  const res = await fetch("http://127.0.0", {
    cache: "no-store",
  });
  const data: { text: string } = await res.json();

  return (
    <main className="flex min-h-screen items-center justify-center">
      <h1 className="text-2xl font-bold">{data.text}</h1>
    </main>
  );
}
