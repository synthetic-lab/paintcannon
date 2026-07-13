import { createReadStream, promises as fs, statSync } from "node:fs";
import path from "node:path";
import { Readable } from "node:stream";
import { parse } from "shell-quote";

const CHARACTER_PLACEHOLDER = "_";

const IMAGE_MIME_TYPES_BY_EXTENSION = {
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".gif": "image/gif",
} as const;

export type PaintFileType =
  (typeof IMAGE_MIME_TYPES_BY_EXTENSION)[keyof typeof IMAGE_MIME_TYPES_BY_EXTENSION];

export type PaintFile = {
  readonly name: string;
  readonly size: number;
  readonly type: PaintFileType;
  readonly lastModified: number;
  readonly webkitRelativePath: string;
  arrayBuffer(): Promise<ArrayBuffer>;
  bytes(): Promise<Uint8Array<ArrayBuffer>>;
  text(): Promise<string>;
  stream(): ReadableStream<Uint8Array>;
};

export type PaintFileList = {
  readonly [index: number]: PaintFile;
  readonly length: number;
  item(index: number): PaintFile | null;
  [Symbol.iterator](): Iterator<PaintFile>;
};

export function filesFromTerminalPaste(input: string): PaintFileList {
  const filePaths = parseImagePaths(input);
  if (filePaths === null) {
    return createPaintFileList([]);
  }

  const files: PaintFile[] = [];
  for (const filePath of filePaths) {
    const resolvedPath = path.resolve(filePath);
    let metadata;
    try {
      metadata = statSync(resolvedPath);
    } catch {
      return createPaintFileList([]);
    }
    if (!metadata.isFile()) {
      return createPaintFileList([]);
    }

    const type = imageTypeFromPath(resolvedPath);
    if (type === undefined) {
      return createPaintFileList([]);
    }
    const bytes = async (): Promise<Uint8Array<ArrayBuffer>> =>
      Uint8Array.from(await fs.readFile(resolvedPath));
    files.push(
      Object.freeze({
        name: path.basename(resolvedPath),
        size: metadata.size,
        type,
        lastModified: Math.trunc(metadata.mtimeMs),
        webkitRelativePath: "",
        async arrayBuffer(): Promise<ArrayBuffer> {
          return (await bytes()).buffer;
        },
        bytes,
        text: () => fs.readFile(resolvedPath, "utf8"),
        stream: () => Readable.toWeb(createReadStream(resolvedPath)) as ReadableStream<Uint8Array>,
      }),
    );
  }
  return createPaintFileList(files);
}

export function parseImagePaths(input: string): string[] | null {
  const dequoted = dequote(input);
  const filePaths = separateFilePaths(dequoted);
  const sanitizedFilePaths = filePaths.map(filePath => sanitizeFilePath(filePath));
  const imagePaths: string[] = [];
  for (const filePath of sanitizedFilePaths) {
    if (isImagePath(filePath)) {
      imagePaths.push(filePath);
    } else {
      return null;
    }
  }
  return imagePaths;
}

export function separateFilePaths(input: string): string[] {
  const placeholderInput = replaceInputWithSafeCharacters(input);
  const parsedPlaceholderInput = parse(placeholderInput);
  const filePaths: string[] = [];
  let cursor = 0;
  for (const separatedPlaceholderPath of parsedPlaceholderInput) {
    if (typeof separatedPlaceholderPath === "string") {
      filePaths.push(input.slice(cursor, cursor + separatedPlaceholderPath.length));
      cursor += separatedPlaceholderPath.length + 1;
    }
  }
  return filePaths.flatMap(filePath => filePath.split("\n"));
}

function replaceInputWithSafeCharacters(input: string): string {
  let escaped = false;
  let sanitized = "";
  for (const character of input) {
    if (character === "\n" || character === "\r") {
      sanitized += character;
    } else if (character === "\\") {
      escaped = !escaped;
      sanitized += CHARACTER_PLACEHOLDER;
    } else if (character === " " && !escaped) {
      sanitized += " ";
      escaped = false;
    } else {
      sanitized += CHARACTER_PLACEHOLDER;
      escaped = false;
    }
  }
  return sanitized;
}

function sanitizeFilePath(filePath: string): string {
  return filePath.trim().replace(/\\(.)/g, "$1");
}

function dequote(input: string): string {
  return dequoteType(dequoteType(input, "'"), '"');
}

function dequoteType(input: string, quoteType: string): string {
  let inQuote = false;
  const characters: string[] = [];
  for (const character of input) {
    if (character === quoteType) {
      inQuote = !inQuote;
      continue;
    }

    if (!inQuote) {
      characters.push(character);
      continue;
    }

    switch (character) {
      case " ":
        characters.push("\\", " ");
        break;
      case "\n":
        characters.push("\\", "n");
        break;
      case "\r":
        characters.push("\\", "\r");
        break;
      default:
        characters.push(character);
    }
  }
  return characters.join("");
}

function isImagePath(filePath: string): boolean {
  const parsed = path.parse(filePath.trim());
  return (parsed.dir !== "" || parsed.ext !== "") && imageTypeFromPath(filePath) !== undefined;
}

function imageTypeFromPath(filePath: string): PaintFileType | undefined {
  const extension = path.extname(filePath).toLowerCase();
  return IMAGE_MIME_TYPES_BY_EXTENSION[extension as keyof typeof IMAGE_MIME_TYPES_BY_EXTENSION];
}

function createPaintFileList(files: readonly PaintFile[]): PaintFileList {
  const entries = Object.freeze(Array.from(files));
  const fileList: PaintFileList = {
    length: entries.length,
    item: index => entries[index] ?? null,
    [Symbol.iterator]: () => entries[Symbol.iterator](),
  };
  for (const [index, file] of entries.entries()) {
    Object.defineProperty(fileList, index, {
      configurable: false,
      enumerable: true,
      value: file,
      writable: false,
    });
  }
  return Object.freeze(fileList);
}
