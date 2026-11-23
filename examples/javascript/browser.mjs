import { makeReport } from "./author.mjs";

const form = document.querySelector("form");
const input = form.querySelector("input");
const button = form.querySelector("button");
const status = document.querySelector("[role=status]");
form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const [image] = input.files;
  if (!image) return;
  button.disabled = true;
  status.textContent = "Creating document…";
  try {
    const bytes = await makeReport(new Uint8Array(await image.arrayBuffer()));
    if (!(bytes instanceof Uint8Array)) throw new TypeError("Expected DOCX bytes");
    const blob = new Blob([bytes], {
      type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    });
    const url = URL.createObjectURL(blob);
    const link = Object.assign(document.createElement("a"), { href: url, download: "report.docx" });
    document.body.append(link);
    link.click();
    link.remove();
    // Leave the Blob available while the browser starts the download.
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
    status.textContent = "Download started.";
  } catch (error) {
    status.textContent = `Could not create DOCX: ${error.message}`;
  } finally {
    button.disabled = false;
  }
});
