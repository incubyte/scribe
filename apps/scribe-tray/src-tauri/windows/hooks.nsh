!define VC_RUNTIME_KEY "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"
!define VC_REDIST_URL "https://aka.ms/vs/17/release/vc_redist.x64.exe"

!macro NSIS_HOOK_POSTINSTALL
  SetRegView 64
  ReadRegDWORD $0 HKLM "${VC_RUNTIME_KEY}" "Installed"
  ${If} $0 != 1
    DetailPrint "Installing the Microsoft Visual C++ runtime Scribe needs..."
    ExecShellWait "open" "$INSTDIR\resources\vc_redist.x64.exe" "/install /quiet /norestart"
    ReadRegDWORD $0 HKLM "${VC_RUNTIME_KEY}" "Installed"
    ${If} $0 != 1
      MessageBox MB_ICONEXCLAMATION|MB_OK "Scribe needs the Microsoft Visual C++ runtime, which could not be installed. Install it from ${VC_REDIST_URL}, then start Scribe."
    ${EndIf}
  ${EndIf}
  SetRegView lastused
  Delete "$INSTDIR\resources\vc_redist.x64.exe"
!macroend
