import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { PaintClipboardEvent, type PaintFileList } from "../main.ts";
import { parseImagePaths, separateFilePaths } from "../terminal-files.ts";

const temporaryDirectories: string[] = [];

afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { force: true, recursive: true });
  }
});

describe("separateFilePaths", () => {
  it("keeps one path with no special spaces", () => {
    expect(separateFilePaths("/path/to/file.png")).toEqual(["/path/to/file.png"]);
  });

  it("separates multiple paths on regular spaces", () => {
    expect(separateFilePaths("/a.png /b.png /c.png")).toEqual(["/a.png", "/b.png", "/c.png"]);
  });

  it("preserves non-breaking and thin spaces inside paths", () => {
    expect(separateFilePaths("/path/my\u00a0file.png /other/my\u2009file.png")).toEqual([
      "/path/my\u00a0file.png",
      "/other/my\u2009file.png",
    ]);
  });
});

describe("parseImagePaths", () => {
  it("accepts the supported image extensions", () => {
    expect(parseImagePaths("/a.png /b.jpg /c.jpeg /d.webp /e.gif")).toEqual([
      "/a.png",
      "/b.jpg",
      "/c.jpeg",
      "/d.webp",
      "/e.gif",
    ]);
  });

  it("rejects the entire payload when any path is not an image", () => {
    expect(parseImagePaths("/a.png /b.txt")).toBeNull();
  });

  it("removes single and double quotes", () => {
    expect(parseImagePaths("'/path/to/file.png'")).toEqual(["/path/to/file.png"]);
    expect(parseImagePaths('"/path/to/file.png"')).toEqual(["/path/to/file.png"]);
  });

  it("unescapes spaces and punctuation", () => {
    expect(parseImagePaths("/path/to/my\\ file\\(1\\).png")).toEqual(["/path/to/my file(1).png"]);
  });
});

describe("terminal image files", () => {
  it("creates browser-shaped files without eagerly reading their contents", async () => {
    const directory = temporaryDirectory();
    const filePath = path.join(directory, "my image (1).png");
    const contents = Uint8Array.from([0x89, 0x50, 0x4e, 0x47]);
    writeFileSync(filePath, contents);

    const files = pastedFiles(`'${filePath}'`);
    const file = files.item(0);
    if (file === null) {
      throw new Error("expected pasted file");
    }

    expect(files).toHaveLength(1);
    expect(file).toBe(files[0]);
    expect(files.item(1)).toBeNull();
    expect(Array.from(files)).toEqual([files[0]]);
    expect(file).toMatchObject({
      name: "my image (1).png",
      size: contents.length,
      type: "image/png",
    });
    expect(file.lastModified).toBeGreaterThan(0);

    const updatedContents = Uint8Array.from([4, 3, 2, 1]);
    writeFileSync(filePath, updatedContents);
    expect(new Uint8Array(await file.arrayBuffer())).toEqual(updatedContents);
    expect(await readStream(file.stream())).toEqual(updatedContents);
  });

  it("recognizes quoted, escaped, and special-space terminal path forms", () => {
    const directory = temporaryDirectory();
    const regularSpace = path.join(directory, "regular space.webp");
    const nonBreakingSpace = path.join(directory, "non\u00a0breaking.gif");
    const thinSpace = path.join(directory, "thin\u2009space.jpeg");
    for (const filePath of [regularSpace, nonBreakingSpace, thinSpace]) {
      writeFileSync(filePath, "image");
    }

    expect(pastedFiles(`"${regularSpace}"`)[0]?.name).toBe("regular space.webp");
    expect(pastedFiles(escapeTerminalPath(regularSpace))[0]?.name).toBe("regular space.webp");
    expect(pastedFiles(nonBreakingSpace)[0]?.name).toBe("non\u00a0breaking.gif");
    expect(pastedFiles(thinSpace)[0]?.name).toBe("thin\u2009space.jpeg");
  });

  it("creates multiple files only when every parsed path is an accessible image", () => {
    const directory = temporaryDirectory();
    const first = path.join(directory, "first image.png");
    const second = path.join(directory, "second (2).jpg");
    const text = path.join(directory, "notes.txt");
    for (const filePath of [first, second, text]) {
      writeFileSync(filePath, "contents");
    }

    const images = pastedFiles(`'${first}' "${second}"`);
    expect(Array.from(images, file => file.name)).toEqual(["first image.png", "second (2).jpg"]);
    expect(pastedFiles(`'${first}' ${text}`)).toHaveLength(0);
    expect(pastedFiles(path.join(directory, "missing.png"))).toHaveLength(0);
  });
});

function temporaryDirectory(): string {
  const directory = mkdtempSync(path.join(os.tmpdir(), "paintcannon-paste-"));
  temporaryDirectories.push(directory);
  return directory;
}

function escapeTerminalPath(filePath: string): string {
  return filePath.replace(/([ ()])/g, "\\$1");
}

function pastedFiles(input: string): PaintFileList {
  return new PaintClipboardEvent(input).clipboardData.files;
}

async function readStream(stream: ReadableStream<Uint8Array>): Promise<Uint8Array> {
  const chunks: number[] = [];
  for await (const chunk of stream) {
    chunks.push(...chunk);
  }
  return Uint8Array.from(chunks);
}
