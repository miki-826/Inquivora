!macro NSIS_HOOK_POSTINSTALL
  ; 更新時にもショートカットのアイコンを実行ファイルから読み直す。
  ; デスクトップの既存ショートカットも同じアイコンへ更新する。
  SetShellVarContext current
  Delete "$SMPROGRAMS\Inquivora.lnk"
  CreateShortcut "$SMPROGRAMS\Inquivora.lnk" "$INSTDIR\inquivora.exe" "" "$INSTDIR\inquivora.exe" 0
  Delete "$DESKTOP\Inquivora.lnk"
  CreateShortcut "$DESKTOP\Inquivora.lnk" "$INSTDIR\inquivora.exe" "" "$INSTDIR\inquivora.exe" 0
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
