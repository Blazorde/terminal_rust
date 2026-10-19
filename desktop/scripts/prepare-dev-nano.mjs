import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const nanoVersion = "9.2";
const nanoUrl = `https://nano-editor.org/dist/windows/nano-v${nanoVersion}-x64.exe`;
const expectedSha256 =
  "0771be067c73935b2d326dc8658b20e88f7c033da2f590cec82e985282ce3507";
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const projectDirectory = resolve(scriptDirectory, "../src-tauri/resources");
const workspaceDirectory = resolve(scriptDirectory, "../..");
const nanoPath = resolve(projectDirectory, "nano-win64.exe");
const temporaryPath = `${nanoPath}.download`;

if (process.platform === "win32") {
  await mkdir(projectDirectory, { recursive: true });

  let binary;
  try {
    binary = await readFile(nanoPath);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }

  if (!binary || createHash("sha256").update(binary).digest("hex") !== expectedSha256) {
    const response = await fetch(nanoUrl);
    if (!response.ok) {
      throw new Error(`Could not download Nano ${nanoVersion}: HTTP ${response.status}`);
    }

    binary = Buffer.from(await response.arrayBuffer());
    const actualSha256 = createHash("sha256").update(binary).digest("hex");
    if (actualSha256 !== expectedSha256) {
      throw new Error(`Nano ${nanoVersion} checksum mismatch: ${actualSha256}`);
    }

    await writeFile(temporaryPath, binary);
    await rm(nanoPath, { force: true });
    await rename(temporaryPath, nanoPath);
  }

  const targetDirectory = resolve(
    workspaceDirectory,
    process.env.CARGO_TARGET_DIR || "target",
    "debug/resources",
  );
  await mkdir(targetDirectory, { recursive: true });
  await copyFile(nanoPath, resolve(targetDirectory, "nano-win64.exe"));
  console.log(`Nano ${nanoVersion} is ready for the development terminal.`);
}
