; Lumen's Windows installer. Built in CI with:
;   makensis /DVERSION=x.y.z /DSRC=<folder with lumen.exe and libmpv-2.dll> /DOUT=<setup.exe> lumen.nsi
Unicode true
!include "MUI2.nsh"

Name "Lumen"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\Lumen"
InstallDirRegKey HKCU "Software\Lumen" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "..\..\..\build\icon.ico"
!define MUI_UNICON "..\..\..\build\icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\Lumen.exe"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

!define UNINST "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lumen"

Section "Lumen"
  ; Close a running copy so its files can be replaced.
  nsExec::Exec 'taskkill /IM Lumen.exe /F'
  SetOutPath "$INSTDIR"
  File /oname=Lumen.exe "${SRC}\lumen.exe"
  File "${SRC}\libmpv-2.dll"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateShortcut "$SMPROGRAMS\Lumen.lnk" "$INSTDIR\Lumen.exe"
  CreateShortcut "$DESKTOP\Lumen.lnk" "$INSTDIR\Lumen.exe"
  WriteRegStr HKCU "Software\Lumen" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST}" "DisplayName" "Lumen"
  WriteRegStr HKCU "${UNINST}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST}" "Publisher" "wumgub"
  WriteRegStr HKCU "${UNINST}" "DisplayIcon" "$INSTDIR\Lumen.exe"
  WriteRegStr HKCU "${UNINST}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${UNINST}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  ; The library (in %APPDATA%\Lumen) is left alone.
  Delete "$INSTDIR\Lumen.exe"
  Delete "$INSTDIR\libmpv-2.dll"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Lumen.lnk"
  Delete "$DESKTOP\Lumen.lnk"
  DeleteRegKey HKCU "${UNINST}"
  DeleteRegKey HKCU "Software\Lumen"
SectionEnd
