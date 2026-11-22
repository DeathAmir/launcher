import { fs, invoke, path } from "@tauri-apps/api";
import { download } from "tauri-plugin-upload-api";

const VOICE_URL = "https://irautox.ir/iRxVoice.zip";
const MARKER_NAME = ".iRxVoice.sha256";

const sha256 = async (data: Uint8Array): Promise<string> => {
  const digest = await crypto.subtle.digest("SHA-256", data);
  return Array.from(new Uint8Array(digest)).map((value) => value.toString(16).padStart(2, "0")).join("");
};

export const ensureIrxVoice = async (
  gtasaPath: string,
  onProgress: (value: number) => void,
  confirmUpdate: () => Promise<boolean>
): Promise<boolean> => {
  if (!(await fs.exists(await path.join(gtasaPath, "gta_sa.exe")))) {
    throw new Error("مسیر انتخاب‌شده، پوشه معتبر GTA: San Andreas نیست.");
  }

  const dataDir = await path.appLocalDataDir();
  const archivePath = await path.join(dataDir, "iRxVoice.zip");
  const markerPath = await path.join(gtasaPath, MARKER_NAME);
  const pluginPath = await path.join(gtasaPath, "iRxVoice.asi");

  let downloadedBytes = 0;
  await download(VOICE_URL, archivePath, (received, total) => {
    downloadedBytes += received;
    onProgress(total > 0 ? Math.min(100, downloadedBytes * 100 / total) : 0);
  });

  const archiveHash = await sha256(await fs.readBinaryFile(archivePath));
  const hasPlugin = await fs.exists(pluginPath);
  const installedHash = await fs.exists(markerPath)
    ? (await fs.readTextFile(markerPath)).trim().toLowerCase()
    : "";

  if (hasPlugin && installedHash === archiveHash.toLowerCase()) {
    onProgress(100);
    await fs.removeFile(archivePath).catch(() => {});
    return true;
  }

  if (hasPlugin && !(await confirmUpdate())) {
    await fs.removeFile(archivePath).catch(() => {});
    return false;
  }

  await invoke("install_irx_voice_archive", { archivePath, gtasaPath });

  if (!(await fs.exists(pluginPath))) {
    throw new Error("نصب فایل iRxVoice.asi تأیید نشد.");
  }

  await fs.writeTextFile(markerPath, archiveHash);
  onProgress(100);
  await fs.removeFile(archivePath).catch(() => {});
  return true;
};
