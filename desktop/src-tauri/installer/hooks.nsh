; Installazione e disinstallazione forzate: se Aurora e' aperta (anche nascosta nel vassoio) la si chiude da soli,
; (solo il programma, non i suoi figli: l'installatore lanciato da Aurora non deve morire con lei) senza chiedere e senza fermarsi con «Aurora e' in esecuzione!». Il core riparte al prossimo avvio.

; Una sola chiusura non bastava: se Aurora resta viva rifa' partire il suo rigctld (si ricollega alla radio) e la DLL di Hamlib resta
; bloccata ("Errore nell'apertura del file in scrittura: libhamlib-4.dll"). Si ripete finche' Aurora e il suo rigctld sono spariti
; (al massimo 10 s); se non ci si riesce (Aurora avviata come amministratore) si chiede di chiuderla a mano.
!macro AuroraStopRunning
  StrCpy $1 0
  aurora_stop_retry:
  ; prima la via piu' semplice, senza PowerShell (che su alcuni PC e' lento o bloccato)
  nsExec::Exec /TIMEOUT=8000 'taskkill /F /IM aurora-desktop.exe'
  Pop $0
  ; mai piu' di 15 s: su alcuni PC PowerShell parte lentissimo o resta appeso, e l'installazione sembrava ferma all'inizio
  nsExec::Exec /TIMEOUT=15000 `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -Command "for ($$i = 0; $$i -lt 20; $$i++) { Get-Process aurora-desktop -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue; Get-Process rigctl* -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like '$INSTDIR\hamlib\*' } | Stop-Process -Force -ErrorAction SilentlyContinue; Start-Sleep -Milliseconds 500; if (-not (Get-Process aurora-desktop -ErrorAction SilentlyContinue) -and -not (Get-Process rigctl* -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like '$INSTDIR\hamlib\*' })) { exit 0 } }; exit 1"`
  Pop $0
  StrCmp $0 "0" aurora_stop_done
  ; PowerShell non ha risposto in tempo o non parte: non si resta fermi, taskkill ha gia' chiuso Aurora
  StrCmp $0 "timeout" aurora_stop_done
  StrCmp $0 "error" aurora_stop_done
  IntOp $1 $1 + 1
  IntCmp $1 3 aurora_stop_done
  MessageBox MB_OK|MB_ICONEXCLAMATION "Aurora e' ancora aperta e non si lascia chiudere (forse e' stata avviata come amministratore).$\r$\nChiudila dal vassoio (tasto destro sull'icona, Esci), poi premi OK."
  Goto aurora_stop_retry
  aurora_stop_done:
!macroend

; Hamlib incluso (cartella "hamlib" accanto al programma): prima di copiare i file nuovi si ferma l'eventuale rigctld di Aurora rimasto vivo
; (solo quello lanciato da questa cartella, non i rigctld che l'utente usa per altri programmi) e si toglie la cartella vecchia,
; cosi' un aggiornamento non lascia file di versioni precedenti.
!macro AuroraCleanHamlib
  ; si ripete finche' la cartella sparisce davvero: un rigctld rimasto orfano (Aurora chiusa mentre lo riavviava) teneva bloccata la DLL
  StrCpy $2 0
  aurora_hamlib_retry:
  ; Get-Process al posto di Get-CimInstance: WMI su certi PC impiega minuti; e comunque al massimo 8 s per giro
  nsExec::Exec /TIMEOUT=8000 `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -Command "Get-Process rigctl* -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like '$INSTDIR\hamlib\*' } | Stop-Process -Force -ErrorAction SilentlyContinue"`
  Pop $0
  Sleep 400
  RMDir /r "$INSTDIR\hamlib"
  IfFileExists "$INSTDIR\hamlib\*.*" 0 aurora_hamlib_done
  IntOp $2 $2 + 1
  IntCmp $2 6 aurora_hamlib_done aurora_hamlib_retry aurora_hamlib_done
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
