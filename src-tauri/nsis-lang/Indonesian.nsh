; Indonesian installer text for Mem Reduct.
;
; Tauri's NSIS bundle ships translations for 21 languages but not Indonesian (nor
; Polish/Vietnamese/Thai). Listing a language in `bundle > windows > nsis >
; languages` without a matching file here is NOT an error — Tauri only logs a
; warning and silently drops the language from the installer, which is why
; these files exist.
;
; Keys, placeholders (`{{product_name}}`, `${PRODUCTNAME}`, `${VERSION}`, `$0`,
; `$1`, `$R4`, `$\n`) and the `choowHowToInstall` typo must match Tauri's
; English.nsh exactly — it is the contract, not a suggestion.
;
; Source of truth:
; https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle/windows/nsis/languages/English.nsh

LangString addOrReinstall ${LANG_INDONESIAN} "Tambah / Pasang ulang komponen"
LangString alreadyInstalled ${LANG_INDONESIAN} "Sudah terpasang"
LangString alreadyInstalledLong ${LANG_INDONESIAN} "${PRODUCTNAME} ${VERSION} sudah terpasang. Pilih tindakan yang ingin Anda lakukan lalu klik Berikutnya untuk melanjutkan."
LangString appRunning ${LANG_INDONESIAN} "{{product_name}} sedang berjalan! Tutup dulu aplikasinya lalu coba lagi."
LangString appRunningOkKill ${LANG_INDONESIAN} "{{product_name}} sedang berjalan!$\nKlik OK untuk menutupnya"
LangString chooseMaintenanceOption ${LANG_INDONESIAN} "Pilih tindakan pemeliharaan yang ingin dilakukan."
LangString choowHowToInstall ${LANG_INDONESIAN} "Pilih cara Anda ingin memasang ${PRODUCTNAME}."
LangString createDesktop ${LANG_INDONESIAN} "Buat pintasan di desktop"
LangString dontUninstall ${LANG_INDONESIAN} "Jangan copot pemasangan"
LangString dontUninstallDowngrade ${LANG_INDONESIAN} "Jangan copot pemasangan (penurunan versi tanpa mencopot pemasangan dinonaktifkan pada pemasang ini)"
LangString failedToKillApp ${LANG_INDONESIAN} "Gagal menutup {{product_name}}. Tutup dulu secara manual lalu coba lagi"
LangString installingWebview2 ${LANG_INDONESIAN} "Memasang WebView2..."
LangString newerVersionInstalled ${LANG_INDONESIAN} "Versi ${PRODUCTNAME} yang lebih baru sudah terpasang! Memasang versi lama tidak disarankan. Jika Anda benar-benar ingin memasang versi lama ini, sebaiknya copot dulu versi yang sekarang. Pilih tindakan yang ingin Anda lakukan lalu klik Berikutnya untuk melanjutkan."
LangString older ${LANG_INDONESIAN} "lebih lama"
LangString olderOrUnknownVersionInstalled ${LANG_INDONESIAN} "Versi $R4 dari ${PRODUCTNAME} terpasang di sistem Anda. Sebaiknya copot versi yang sekarang sebelum memasang. Pilih tindakan yang ingin Anda lakukan lalu klik Berikutnya untuk melanjutkan."
LangString silentDowngrades ${LANG_INDONESIAN} "Penurunan versi dinonaktifkan pada pemasang ini, pemasangan senyap tidak dapat dilanjutkan. Gunakan pemasang dengan antarmuka grafis.$\n"
LangString unableToUninstall ${LANG_INDONESIAN} "Gagal mencopot pemasangan!"
LangString uninstallApp ${LANG_INDONESIAN} "Copot pemasangan ${PRODUCTNAME}"
LangString uninstallBeforeInstalling ${LANG_INDONESIAN} "Copot pemasangan sebelum memasang"
LangString unknown ${LANG_INDONESIAN} "tidak diketahui"
LangString webview2AbortError ${LANG_INDONESIAN} "Gagal memasang WebView2! Aplikasi tidak dapat berjalan tanpanya. Coba mulai ulang pemasangnya."
LangString webview2DownloadError ${LANG_INDONESIAN} "Kesalahan: Gagal mengunduh WebView2 - $0"
LangString webview2DownloadSuccess ${LANG_INDONESIAN} "Bootstrapper WebView2 berhasil diunduh"
LangString webview2Downloading ${LANG_INDONESIAN} "Mengunduh bootstrapper WebView2..."
LangString webview2InstallError ${LANG_INDONESIAN} "Kesalahan: Pemasangan WebView2 gagal dengan kode keluar $1"
LangString webview2InstallSuccess ${LANG_INDONESIAN} "WebView2 berhasil dipasang"
LangString deleteAppData ${LANG_INDONESIAN} "Hapus data aplikasi"
