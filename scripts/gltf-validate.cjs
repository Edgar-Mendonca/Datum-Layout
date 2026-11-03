// Validates .glb files with the Khronos glTF validator (npm i gltf-validator; scripts/verify.sh).
const v = require("gltf-validator"), fs = require("fs"), path = require("path");
(async () => { for (const f of process.argv.slice(2)) { const r = await v.validateBytes(new Uint8Array(fs.readFileSync(f))); const i = r.issues;
  console.log(`${i.numErrors ? "FAIL" : "ok  "} ${path.basename(f)}: ${i.numErrors} errors, ${i.numWarnings} warnings, ${i.numInfos} infos; ${r.info.totalVertexCount} vertices, ${r.info.totalTriangleCount} triangles`);
  for (const m of i.messages.filter(m => m.severity <= 1).slice(0, 4)) console.log(`     ${m.code} ${m.message} ${m.pointer ?? ""}`); } })();
