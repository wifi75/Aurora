; Installazione e disinstallazione forzate: se Aurora e' aperta (anche nascosta nel vassoio) la si chiude da soli,
; (solo il programma, non i suoi figli: l'installatore lanciato da Aurora non deve morire con lei) senza chiedere e senza fermarsi con «Aurora e' in esecuzione!». Il core riparte al prossimo avvio.

; Veloce prima di tutto: solo taskkill (pochi decimi di secondo). PowerShell, che su alcuni PC aziendali parte lentissimo o resta appeso,
; si usa soltanto se qualcosa e' ancora bloccato, e con un tempo massimo breve.
!macro AuroraStopRunning
  nsExec::Exec /TIMEOUT=6000 'taskkill /F /IM aurora-desktop.exe'
  Pop $0
  ; anche il core avviato da solo (aurora.exe): il 2026-11-09 un 0.11.155 rimasto acceso teneva la porta e la finestra dopo l'aggiornamento
  nsExec::Exec /TIMEOUT=6000 'taskkill /F /IM aurora.exe'
  Pop $0
  ; il tempo di lasciare i file a Windows
  Sleep 500
!macroend

; Hamlib incluso (cartella "hamlib" accanto al programma): si toglie la cartella vecchia, cosi' un aggiornamento non lascia file di versioni
; precedenti. Se un rigctld di Aurora rimasto vivo tiene bloccata la DLL ("libhamlib-4.dll"), solo allora lo si ferma (solo quello di questa
; cartella con PowerShell, prima ancora con taskkill sul rigctld che tiene caricata libhamlib-4.dll) e si riprova; al massimo quattro volte.
!macro AuroraCleanHamlib
  RMDir /r "$INSTDIR\hamlib"
  IfFileExists "$INSTDIR\hamlib\*.*" 0 aurora_hamlib_done
  StrCpy $2 0
  aurora_hamlib_retry:
  ; prima senza PowerShell: il rigctld che ha caricato la DLL di Hamlib (quello di Aurora rimasto vivo dopo la chiusura del programma)
  nsExec::Exec /TIMEOUT=6000 'taskkill /F /IM rigctld.exe /FI "MODULES eq libhamlib-4.dll"'
  Pop $0
  Sleep 800
  RMDir /r "$INSTDIR\hamlib"
  IfFileExists "$INSTDIR\hamlib\*.*" 0 aurora_hamlib_done
  nsExec::Exec /TIMEOUT=6000 `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -Command "Get-Process rigctl* -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like '$INSTDIR\hamlib\*' } | Stop-Process -Force -ErrorAction SilentlyContinue"`
  Pop $0
  Sleep 400
  RMDir /r "$INSTDIR\hamlib"
  IfFileExists "$INSTDIR\hamlib\*.*" 0 aurora_hamlib_done
  IntOp $2 $2 + 1
  IntCmp $2 4 aurora_hamlib_done aurora_hamlib_retry aurora_hamlib_done
  aurora_hamlib_done:
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro AuroraStopRunning
  !insertmacro AuroraCleanHamlib
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro AuroraStopRunning
  !insertmacro AuroraCleanHamlib
!macroend
