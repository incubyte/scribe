export type ColorScheme = "light" | "dark";

const HEX_COLOR = /^#([0-9a-f]{6})$/i;
const ROOT_BLOCK = /:root\s*\{([^}]*)\}/g;
const DARK_QUERY = "@media (prefers-color-scheme: dark)";

export function contrastRatio(foreground: string, background: string): number {
  const [lighter, darker] = [relativeLuminance(foreground), relativeLuminance(background)].sort((a, b) => b - a);
  return (lighter + 0.05) / (darker + 0.05);
}

export function themeTokens(html: string, scheme: ColorScheme): Record<string, string> {
  const [lightBlock = "", darkBlock = ""] = rootBlocks(html);
  const light = tokensIn(lightBlock);
  return scheme === "light" ? light : { ...light, ...tokensIn(darkBlock) };
}

function rootBlocks(html: string): string[] {
  const styles = html.slice(html.indexOf("<style>"), html.indexOf("</style>"));
  const darkStart = styles.indexOf(DARK_QUERY);
  const lightPart = styles.slice(0, darkStart);
  const darkPart = styles.slice(darkStart, styles.indexOf("@media", darkStart + DARK_QUERY.length));
  return [lightPart, darkPart].map((part) => Array.from(part.matchAll(ROOT_BLOCK), (match) => match[1]).join("\n"));
}

function tokensIn(block: string): Record<string, string> {
  return Object.fromEntries(block.split(";").flatMap(customProperty));
}

function customProperty(declaration: string): [string, string][] {
  const separator = declaration.indexOf(":");
  const name = declaration.slice(0, separator).trim();
  return name.startsWith("--") ? [[name, declaration.slice(separator + 1).trim()]] : [];
}

function relativeLuminance(hex: string): number {
  const digits = HEX_COLOR.exec(hex)?.[1];
  if (!digits) {
    throw new Error("Not a six-digit hex colour: " + hex);
  }
  const [red, green, blue] = [0, 2, 4].map((start) => linearChannel(parseInt(digits.slice(start, start + 2), 16)));
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function linearChannel(channel: number): number {
  const unit = channel / 255;
  return unit <= 0.03928 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
}
