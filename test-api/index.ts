const server = Bun.serve({
  port: Bun.argv[2] || 8080,
  // `routes` requires Bun v1.2.3+
  fetch(req) {
    console.log(`request received to PORT ${Bun.argv[2]}`);

    const url = new URL(req.url);

    if (url.pathname === "/health") {
      return new Response(JSON.stringify({ status: "ok", from: Bun.argv[2] }), {
        headers: { "Content-Type": "application/json" },
      });
    } else if (url.pathname === "/data") {
      const data = {
        message: "This is some sample data.",
        timestamp: Date.now(),
        from: Bun.argv[2],
      };
      return new Response(JSON.stringify(data), {
        headers: { "Content-Type": "application/json" },
      });
    } else {
      return new Response("404 Not Found", { status: 404 });
    }
  },
});

console.log(`Listening on ${server.url}`);
console.log("Port " + Bun.argv[2]);
