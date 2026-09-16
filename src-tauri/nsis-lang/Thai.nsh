; Thai installer text for Mem Reduct.
;
; Tauri's NSIS bundle ships translations for 21 languages but not Thai (nor
; Polish/Vietnamese/Indonesian). Listing a language in `bundle > windows > nsis >
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

LangString addOrReinstall ${LANG_THAI} "เพิ่ม / ติดตั้งองค์ประกอบใหม่"
LangString alreadyInstalled ${LANG_THAI} "ติดตั้งไว้แล้ว"
LangString alreadyInstalledLong ${LANG_THAI} "ติดตั้ง ${PRODUCTNAME} ${VERSION} ไว้แล้ว โปรดเลือกการทำงานที่ต้องการแล้วคลิกถัดไปเพื่อดำเนินการต่อ"
LangString appRunning ${LANG_THAI} "{{product_name}} กำลังทำงานอยู่! โปรดปิดโปรแกรมก่อนแล้วลองใหม่"
LangString appRunningOkKill ${LANG_THAI} "{{product_name}} กำลังทำงานอยู่!$\nคลิกตกลงเพื่อปิดโปรแกรม"
LangString chooseMaintenanceOption ${LANG_THAI} "เลือกการทำงานด้านการบำรุงรักษาที่ต้องการ"
LangString choowHowToInstall ${LANG_THAI} "เลือกวิธีที่คุณต้องการติดตั้ง ${PRODUCTNAME}"
LangString createDesktop ${LANG_THAI} "สร้างทางลัดบนเดสก์ท็อป"
LangString dontUninstall ${LANG_THAI} "ไม่ต้องถอนการติดตั้ง"
LangString dontUninstallDowngrade ${LANG_THAI} "ไม่ต้องถอนการติดตั้ง (ตัวติดตั้งนี้ไม่รองรับการติดตั้งเวอร์ชันเก่ากว่าโดยไม่ถอนการติดตั้ง)"
LangString failedToKillApp ${LANG_THAI} "ปิด {{product_name}} ไม่สำเร็จ โปรดปิดโปรแกรมด้วยตนเองแล้วลองใหม่"
LangString installingWebview2 ${LANG_THAI} "กำลังติดตั้ง WebView2..."
LangString newerVersionInstalled ${LANG_THAI} "ติดตั้ง ${PRODUCTNAME} เวอร์ชันใหม่กว่าไว้แล้ว! ไม่แนะนำให้ติดตั้งเวอร์ชันเก่ากว่า หากต้องการติดตั้งเวอร์ชันเก่านี้จริง ๆ ควรถอนการติดตั้งเวอร์ชันปัจจุบันก่อน โปรดเลือกการทำงานที่ต้องการแล้วคลิกถัดไปเพื่อดำเนินการต่อ"
LangString older ${LANG_THAI} "เก่ากว่า"
LangString olderOrUnknownVersionInstalled ${LANG_THAI} "ระบบมี ${PRODUCTNAME} เวอร์ชัน $R4 ติดตั้งอยู่ แนะนำให้ถอนการติดตั้งเวอร์ชันปัจจุบันก่อนติดตั้งใหม่ โปรดเลือกการทำงานที่ต้องการแล้วคลิกถัดไปเพื่อดำเนินการต่อ"
LangString silentDowngrades ${LANG_THAI} "ตัวติดตั้งนี้ไม่รองรับการติดตั้งเวอร์ชันเก่ากว่า จึงไม่สามารถดำเนินการแบบเงียบได้ โปรดใช้ตัวติดตั้งแบบมีหน้าจอ$\n"
LangString unableToUninstall ${LANG_THAI} "ถอนการติดตั้งไม่สำเร็จ!"
LangString uninstallApp ${LANG_THAI} "ถอนการติดตั้ง ${PRODUCTNAME}"
LangString uninstallBeforeInstalling ${LANG_THAI} "ถอนการติดตั้งก่อนติดตั้ง"
LangString unknown ${LANG_THAI} "ไม่ทราบ"
LangString webview2AbortError ${LANG_THAI} "ติดตั้ง WebView2 ไม่สำเร็จ! โปรแกรมไม่สามารถทำงานได้หากไม่มีองค์ประกอบนี้ โปรดลองเริ่มตัวติดตั้งใหม่"
LangString webview2DownloadError ${LANG_THAI} "ข้อผิดพลาด: ดาวน์โหลด WebView2 ไม่สำเร็จ - $0"
LangString webview2DownloadSuccess ${LANG_THAI} "ดาวน์โหลดตัวติดตั้ง WebView2 สำเร็จ"
LangString webview2Downloading ${LANG_THAI} "กำลังดาวน์โหลดตัวติดตั้ง WebView2..."
LangString webview2InstallError ${LANG_THAI} "ข้อผิดพลาด: ติดตั้ง WebView2 ไม่สำเร็จ รหัสออก $1"
LangString webview2InstallSuccess ${LANG_THAI} "ติดตั้ง WebView2 สำเร็จ"
LangString deleteAppData ${LANG_THAI} "ลบข้อมูลของโปรแกรม"
