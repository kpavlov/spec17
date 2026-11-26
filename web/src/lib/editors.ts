// (WEB-08) open in an editor: `scheme://file/<abs path>:<line>` (VS Code family, Zed), JetBrains `open?file=&line=`, or the editor's own form
const fileUrl = (scheme: string) => (f: string, l: number) => `${scheme}://file${f}:${l}`;
const jetbrains = (scheme: string) => (f: string, l: number) => `${scheme}://open?file=${encodeURIComponent(f)}&line=${l}`;
export const EDITORS: Record<string, (file: string, line: number) => string> = {
  IntelliJ: jetbrains("idea"),
  WebStorm: jetbrains("webstorm"),
  PyCharm: jetbrains("pycharm"),
  GoLand: jetbrains("goland"),
  RustRover: jetbrains("rustrover"),
  CLion: jetbrains("clion"),
  PhpStorm: jetbrains("phpstorm"),
  Rider: jetbrains("rider"),
  "Android Studio": jetbrains("androidstudio"),
  "VS Code": fileUrl("vscode"),
  "VS Code Insiders": fileUrl("vscode-insiders"),
  VSCodium: fileUrl("vscodium"),
  Cursor: fileUrl("cursor"),
  Windsurf: fileUrl("windsurf"),
  Zed: fileUrl("zed"),
  Sublime: (f, l) => `subl://open?url=${encodeURIComponent("file://" + f)}&line=${l}`,
  TextMate: (f, l) => `txmt://open?url=${encodeURIComponent("file://" + f)}&line=${l}`,
  Xcode: (f, l) => `xcode://open?file=${encodeURIComponent(f)}&line=${l}`,
};
