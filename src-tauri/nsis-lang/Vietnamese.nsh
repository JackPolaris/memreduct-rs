; Vietnamese installer text for Mem Reduct.
;
; Tauri's NSIS bundle ships translations for 21 languages but not Vietnamese (nor
; Polish/Thai/Indonesian). Listing a language in `bundle > windows > nsis >
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

LangString addOrReinstall ${LANG_VIETNAMESE} "Thêm / Cài đặt lại thành phần"
LangString alreadyInstalled ${LANG_VIETNAMESE} "Đã cài đặt"
LangString alreadyInstalledLong ${LANG_VIETNAMESE} "${PRODUCTNAME} ${VERSION} đã được cài đặt. Chọn thao tác bạn muốn thực hiện rồi bấm Tiếp theo để tiếp tục."
LangString appRunning ${LANG_VIETNAMESE} "{{product_name}} đang chạy! Vui lòng đóng ứng dụng rồi thử lại."
LangString appRunningOkKill ${LANG_VIETNAMESE} "{{product_name}} đang chạy!$\nBấm OK để đóng ứng dụng"
LangString chooseMaintenanceOption ${LANG_VIETNAMESE} "Chọn thao tác bảo trì cần thực hiện."
LangString choowHowToInstall ${LANG_VIETNAMESE} "Chọn cách bạn muốn cài đặt ${PRODUCTNAME}."
LangString createDesktop ${LANG_VIETNAMESE} "Tạo lối tắt trên màn hình"
LangString dontUninstall ${LANG_VIETNAMESE} "Không gỡ cài đặt"
LangString dontUninstallDowngrade ${LANG_VIETNAMESE} "Không gỡ cài đặt (bản cài đặt này không cho phép hạ cấp mà không gỡ cài đặt)"
LangString failedToKillApp ${LANG_VIETNAMESE} "Không thể đóng {{product_name}}. Vui lòng đóng ứng dụng bằng tay rồi thử lại"
LangString installingWebview2 ${LANG_VIETNAMESE} "Đang cài đặt WebView2..."
LangString newerVersionInstalled ${LANG_VIETNAMESE} "Phiên bản ${PRODUCTNAME} mới hơn đã được cài đặt! Không nên cài đặt phiên bản cũ hơn. Nếu bạn thực sự muốn cài phiên bản cũ này, tốt hơn hết là gỡ phiên bản hiện tại trước. Chọn thao tác bạn muốn thực hiện rồi bấm Tiếp theo để tiếp tục."
LangString older ${LANG_VIETNAMESE} "cũ hơn"
LangString olderOrUnknownVersionInstalled ${LANG_VIETNAMESE} "Hệ thống đang có phiên bản $R4 của ${PRODUCTNAME}. Nên gỡ phiên bản hiện tại trước khi cài đặt. Chọn thao tác bạn muốn thực hiện rồi bấm Tiếp theo để tiếp tục."
LangString silentDowngrades ${LANG_VIETNAMESE} "Bản cài đặt này không cho phép hạ cấp nên không thể tiếp tục ở chế độ ẩn. Vui lòng dùng bản cài đặt có giao diện.$\n"
LangString unableToUninstall ${LANG_VIETNAMESE} "Không thể gỡ cài đặt!"
LangString uninstallApp ${LANG_VIETNAMESE} "Gỡ cài đặt ${PRODUCTNAME}"
LangString uninstallBeforeInstalling ${LANG_VIETNAMESE} "Gỡ cài đặt trước khi cài"
LangString unknown ${LANG_VIETNAMESE} "không rõ"
LangString webview2AbortError ${LANG_VIETNAMESE} "Cài đặt WebView2 thất bại! Ứng dụng không thể chạy nếu thiếu thành phần này. Hãy thử khởi động lại bản cài đặt."
LangString webview2DownloadError ${LANG_VIETNAMESE} "Lỗi: Tải WebView2 thất bại - $0"
LangString webview2DownloadSuccess ${LANG_VIETNAMESE} "Đã tải thành công trình cài đặt WebView2"
LangString webview2Downloading ${LANG_VIETNAMESE} "Đang tải trình cài đặt WebView2..."
LangString webview2InstallError ${LANG_VIETNAMESE} "Lỗi: Cài đặt WebView2 thất bại với mã thoát $1"
LangString webview2InstallSuccess ${LANG_VIETNAMESE} "Đã cài đặt WebView2 thành công"
LangString deleteAppData ${LANG_VIETNAMESE} "Xóa dữ liệu ứng dụng"
