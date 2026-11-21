// Kopiuje statyczne zasoby interfejsu (HTML, CSS) do dist/ obok bundla main.js.
import { copyFileSync, mkdirSync } from "node:fs";

mkdirSync("dist", { recursive: true });
for (const file of ["index.html", "styles.css"]) {
  copyFileSync(`src/${file}`, `dist/${file}`);
}
console.log("assets: index.html, styles.css -> dist/");
