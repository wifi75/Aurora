; Installazione e disinstallazione forzate: se Aurora e' aperta (anche nascosta nel vassoio) la si chiude da soli,
; (solo il programma, non i suoi figli: l'installatore lanciato da Aurora non deve morire con lei) senza chiedere e senza fermarsi con «Aurora e' in esecuzione!». Il core riparte al prossimo avvio.
;
; Il registro dell'installazione: ogni installazione o aggiornamento scrive passi, controlli dell'ambiente, avvisi ed esito in
; installazione.log nella cartella dei dati di Aurora (%APPDATA%\Aurora, o AURORA_DATA_DIR), sovrascrivendo il precedente. Se
; l'installazione non riesce, rapporto-installazione.ps1 manda un rapporto anonimo (se i rapporti non sono spenti in Aurora); se riesce
; con degli avvisi, il rapporto lo manda Aurora al primo avvio (rapporti.rs). I testi del registro sono per chi sviluppa: in italiano e
; solo ASCII (questo file si include senza BOM). Nel registro: «AVVISO:» ed «ERRORE:» all'inizio della riga, «esito:» alla fine.

; questo file si include dalla cartella dei sorgenti (installer\): lo script del rapporto sta qui accanto
!define AURORA_INSTALLER_DIR "${__FILEDIR__}"

; PowerShell da nsExec: senza -InputFormat None resta appeso ad aspettare lo stdin che nsExec tiene aperto (provato il 2026-11-12)
!define AURORA_PS_ARGS "-NoProfile -NonInteractive -InputFormat None -ExecutionPolicy Bypass -WindowStyle Hidden"

; Una riga nel registro (con l'ora davanti); non tocca la segnalazione d'errore di NSIS, che serve a vedere i file non copiati
!macro AuroraScrivi testo
  Push `${testo}`
  Call AuroraScriviRiga
!macroend

; Veloce prima di tutto: solo taskkill (pochi decimi di secondo). PowerShell, che su alcuni PC aziendali parte lentissimo o resta appeso,
; si usa soltanto se qualcosa e' ancora bloccato, e con un tempo massimo breve. Con `registra` = 1 (installazione) i risultati vanno nel registro.
!macro AuroraStopRunning registra
  nsExec::Exec /TIMEOUT=6000 'taskkill /F /IM aurora-desktop.exe'
  Pop $0
  !if ${registra} == 1
    ; taskkill: 0 chiuso, 128 non trovato
    ${If} $0 == 0
      !insertmacro AuroraScrivi "Aurora (o l'agente nel vassoio) era aperta: chiusa"
    ${ElseIf} $0 == 128
      !insertmacro AuroraScrivi "Aurora non era aperta"
    ${Else}
      !insertmacro AuroraScrivi "AVVISO: Aurora aperta e non chiusa da taskkill ($0)"
    ${EndIf}
  !endif
  ; anche il core avviato da solo (aurora.exe): il 2026-11-09 un 0.11.155 rimasto acceso teneva la porta e la finestra dopo l'aggiornamento
  nsExec::Exec /TIMEOUT=6000 'taskkill /F /IM aurora.exe'
  Pop $0
  !if ${registra} == 1
    ${If} $0 == 0
      !insertmacro AuroraScrivi "il core aurora.exe era aperto: chiuso"
    ${ElseIf} $0 != 128
      !insertmacro AuroraScrivi "AVVISO: core aurora.exe aperto e non chiuso da taskkill ($0)"
    ${EndIf}
  !endif
  ; il tempo di lasciare i file a Windows
  Sleep 500
!macroend

; Hamlib incluso (cartella "hamlib" accanto al programma): si toglie la cartella vecchia, cosi' un aggiornamento non lascia file di versioni
; precedenti. Se un rigctld di Aurora rimasto vivo tiene bloccata la DLL ("libhamlib-4.dll"), solo allora lo si ferma (solo quello di questa
; cartella con PowerShell, prima ancora con taskkill sul rigctld che tiene caricata libhamlib-4.dll) e si riprova; al massimo quattro volte.
!macro AuroraCleanHamlib registra
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
  nsExec::Exec /TIMEOUT=6000 `powershell ${AURORA_PS_ARGS} -Command "Get-Process rigctl* -ErrorAction SilentlyContinue | Where-Object { $$_.Path -like '$INSTDIR\hamlib\*' } | Stop-Process -Force -ErrorAction SilentlyContinue"`
  Pop $0
  Sleep 400
  RMDir /r "$INSTDIR\hamlib"
  IfFileExists "$INSTDIR\hamlib\*.*" 0 aurora_hamlib_done
  IntOp $2 $2 + 1
  IntCmp $2 4 aurora_hamlib_done aurora_hamlib_retry aurora_hamlib_done
  aurora_hamlib_done:
  !if ${registra} == 1
    ${If} ${FileExists} "$INSTDIR\hamlib\*.*"
      !insertmacro AuroraScrivi "AVVISO: cartella hamlib vecchia non tolta dopo 4 tentativi (file bloccati da un programma o dall'antivirus)"
    ${EndIf}
  !endif
!macroend

!macro NSIS_HOOK_PREINSTALL
  Call AuroraPrimaDellaCopia
  StrCpy $AuroraPasso "chiusura di Aurora"
  !insertmacro AuroraStopRunning 1
  StrCpy $AuroraPasso "pulizia della cartella hamlib"
  !insertmacro AuroraCleanHamlib 1
  ; al posto di CheckIfAppIsRunning del modello (vedi AuroraControlloProgramma): non ferma piu' l'installazione
  StrCpy $AuroraPasso "controllo dei programmi che tengono aurora-desktop.exe (Restart Manager)"
  !insertmacro AuroraControlloProgramma
!macroend

; Al posto di CheckIfAppIsRunning del modello di Tauri (Restart Manager), che in un aggiornamento silenzioso si fermava se un programma
; estraneo (antivirus, servizio di Windows) teneva aurora-desktop.exe e non si lasciava chiudere: rapporti del 2026-11-12, due PC fermi
; sulla 0.11.238 e 0.11.239 con Aurora gia' chiusa. Qui: si scrive nel registro chi tiene il file, si richiude Aurora e si riprova per
; qualche secondo; se resta tenuto si prova a chiuderlo con il Restart Manager e, se non si lascia chiudere, si va avanti lo stesso: il
; programma vecchio si rinomina (aurora-desktop.exe.old, cosa che Windows permette anche a un file aperto) e il nuovo si copia accanto.
; Il file .old lo toglie Aurora al primo avvio se qui non si e' potuto.
!define /ifndef ERROR_MORE_DATA 234
!macro AuroraControlloProgramma
  StrCpy $R6 0
  aurora_rm_giro:
  StrCpy $R5 0
  !insertmacro RestartManager_StartSession $R0
  ${If} $R0 != ""
    !insertmacro RestartManager_RegisterFile $R0 "$INSTDIR\${MAINBINARYNAME}.exe"
    ${If} $0 = 0
      System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
      ${If} $0 = ${ERROR_MORE_DATA}
        StrCpy $R5 $1
        ; chi lo tiene (il primo dell'elenco): RM_PROCESS_INFO = pid, ora di avvio, nome (256), servizio (64), tipo... 668 byte l'uno
        IntOp $2 $1 * 668
        System::Alloc $2
        Pop $4
        StrCpy $2 $1
        System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i r2 .r2, p r4, *i .r3) i .r0'
        ${If} $0 = 0
          System::Call '*$4(i .r5, i, i, &w256 .r6, &w64 .r7, i .r8)'
          !insertmacro AuroraScrivi "AVVISO: aurora-desktop.exe tenuto da $R5 programmi; il primo: $6 (pid $5, servizio '$7', tipo $8)"
        ${EndIf}
        System::Free $4
        ${If} $R6 >= 4
          System::Call 'RSTRTMGR::RmShutdown(p R0, i ${RmForceShutdown}, p 0) i .r0'
          ${If} $0 = 0
            !insertmacro AuroraScrivi "chiusi con il Restart Manager"
            StrCpy $R5 0
          ${Else}
            !insertmacro AuroraScrivi "AVVISO: non si lasciano chiudere dal Restart Manager ($0): si va avanti rinominando il programma vecchio"
          ${EndIf}
        ${EndIf}
      ${EndIf}
    ${EndIf}
    !insertmacro RestartManager_EndSession $R0
  ${EndIf}
  ${If} $R5 != 0
  ${AndIf} $R6 < 4
    IntOp $R6 $R6 + 1
    nsExec::Exec /TIMEOUT=6000 'taskkill /F /IM ${MAINBINARYNAME}.exe'
    Pop $0
    Sleep 1000
    Goto aurora_rm_giro
  ${EndIf}
  ; il programma vecchio da parte: la copia del nuovo non trova piu' il file occupato
  Delete "$INSTDIR\${MAINBINARYNAME}.exe.old"
  ${If} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    ClearErrors
    Rename "$INSTDIR\${MAINBINARYNAME}.exe" "$INSTDIR\${MAINBINARYNAME}.exe.old"
    ${If} ${Errors}
      !insertmacro AuroraScrivi "AVVISO: programma vecchio non rinominato: la copia prova a sovrascriverlo"
    ${Else}
      Delete "$INSTDIR\${MAINBINARYNAME}.exe.old"
    ${EndIf}
  ${EndIf}
  ClearErrors
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Call AuroraDopoLaCopia
  ${If} $AuroraCausa == "antivirus"
    StrCpy $AuroraPasso "controllo dei file copiati"
    Call AuroraFallita
    Abort
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro AuroraStopRunning 0
  !insertmacro AuroraCleanHamlib 0
!macroend

; Le variabili e le funzioni del registro: si inseriscono nel modello dopo le sue variabili ($PassiveMode, $UpdateMode) e le sue lingue
!macro AuroraFunzioniRegistro
  ; il percorso del registro, vuoto finche' non e' aperto (o se non si puo' scrivere)
  Var AuroraRegistro
  ; la versione installata prima, letta in .onInit (la sezione Install la sovrascrive)
  Var AuroraVersionePrima
  ; il passo in corso: se l'installazione si ferma, il motivo e' questo
  Var AuroraPasso
  ; il motivo dell'errore, scritto prima di ogni Abort conosciuto
  Var AuroraMotivo
  Var AuroraFallitaFatta
  ; la causa per la schermata finale: webview2, versione, file, spazio, antivirus o vuota (altro)
  Var AuroraCausa
  ; il rapporto anonimo: mandato, spenti o non mandato
  Var AuroraRapporto
  ; 1 se sul disco di destinazione c'erano meno di 500 MB liberi
  Var AuroraPocoSpazio
  Var AuroraPowerShell
  Var AuroraScript

  Function AuroraScriviRiga
    Exch $R9
    Push $R8
    Push $R7
    Push $0
    Push $1
    Push $2
    Push $3
    Push $4
    Push $5
    Push $6
    StrCpy $R7 0
    IfErrors 0 +2
      StrCpy $R7 1
    ${If} $AuroraRegistro != ""
      ${GetTime} "" "L" $0 $1 $2 $3 $4 $5 $6
      FileOpen $R8 $AuroraRegistro a
      ${If} $R8 != ""
        FileSeek $R8 0 END
        FileWriteUTF16LE $R8 "[$4:$5:$6] $R9$\r$\n"
        FileClose $R8
      ${EndIf}
    ${EndIf}
    ClearErrors
    ${If} $R7 = 1
      SetErrors
    ${EndIf}
    Pop $6
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Pop $0
    Pop $R7
    Pop $R8
    Pop $R9
  FunctionEnd

  ; Una riga senza l'ora (intestazione ed esito)
  Function AuroraScriviNuda
    Exch $R9
    Push $R8
    ${If} $AuroraRegistro != ""
      FileOpen $R8 $AuroraRegistro a
      ${If} $R8 != ""
        FileSeek $R8 0 END
        FileWriteUTF16LE $R8 "$R9$\r$\n"
        FileClose $R8
      ${EndIf}
    ${EndIf}
    Pop $R8
    Pop $R9
  FunctionEnd

  ; Apre (sovrascrive) il registro e scrive l'intestazione: versioni, modo, Windows, architettura, diritti. Si chiama all'inizio della
  ; prima sezione, quando l'installazione parte davvero: chi apre l'installatore e lo chiude senza installare non tocca il registro di prima.
  Function AuroraApriRegistro
    ${If} $AuroraRegistro != ""
      Return
    ${EndIf}
    Push $0
    Push $1
    Push $2
    Push $3
    Push $4
    Push $5
    Push $6
    ; la cartella dei dati come la sceglie Aurora (lib.rs del programma Windows): AURORA_DATA_DIR, altrimenti %APPDATA%\Aurora
    ReadEnvStr $0 AURORA_DATA_DIR
    ${If} $0 == ""
      ReadEnvStr $0 APPDATA
      StrCpy $0 "$0\Aurora"
    ${EndIf}
    CreateDirectory $0
    FileOpen $1 "$0\installazione.log" w
    ${If} $1 == ""
      Goto aurora_apri_fine
    ${EndIf}
    FileWriteUTF16LE /BOM $1 "Registro dell'installazione di Aurora$\r$\n"
    FileClose $1
    StrCpy $AuroraRegistro "$0\installazione.log"
    ${GetTime} "" "L" $1 $2 $3 $4 $5 $6 $0
    Push "inizio: $3-$2-$1 $5:$6:$0"
    Call AuroraScriviNuda
    Push "versione nuova: ${VERSION} (${ARCH})"
    Call AuroraScriviNuda
    ${If} $AuroraVersionePrima == ""
      Push "versione installata prima: nessuna"
    ${Else}
      Push "versione installata prima: $AuroraVersionePrima"
    ${EndIf}
    Call AuroraScriviNuda
    ${If} ${Silent}
      ${If} $UpdateMode = 1
        StrCpy $0 "aggiornamento silenzioso da Aurora (/S /UPDATE)"
      ${Else}
        StrCpy $0 "installazione silenziosa (/S)"
      ${EndIf}
    ${ElseIf} $PassiveMode = 1
      StrCpy $0 "passiva, senza domande (/P)"
    ${ElseIf} $UpdateMode = 1
      StrCpy $0 "aggiornamento con le finestre (/UPDATE)"
    ${Else}
      StrCpy $0 "normale, con le finestre"
    ${EndIf}
    Push "modo: $0"
    Call AuroraScriviNuda
    ${GetParameters} $0
    Push "parametri: $0"
    Call AuroraScriviNuda
    ; Windows come lo descrive Aurora: «Windows 11 Pro 24H2 (build 26100.1234)»
    SetRegView 64
    ReadRegStr $0 HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion" "ProductName"
    ReadRegStr $1 HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion" "DisplayVersion"
    ReadRegStr $2 HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion" "CurrentBuild"
    ReadRegDWORD $3 HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion" "UBR"
    SetRegView default
    ; dalla build 22000 e' Windows 11, anche se ProductName dice ancora 10
    ${If} $2 >= 22000
      ${WordReplace} "$0" "Windows 10" "Windows 11" "+" $0
    ${EndIf}
    Push "Windows: $0 $1 (build $2.$3)"
    Call AuroraScriviNuda
    ${If} ${IsNativeARM64}
      StrCpy $0 "ARM64"
    ${ElseIf} ${IsNativeAMD64}
      StrCpy $0 "x64"
    ${Else}
      StrCpy $0 "x86 (32 bit)"
    ${EndIf}
    Push "architettura del PC: $0"
    Call AuroraScriviNuda
    UserInfo::GetAccountType
    Pop $0
    Push "diritti dell'utente: $0"
    Call AuroraScriviNuda
    Push "lingua dell'installatore: $LANGUAGE"
    Call AuroraScriviNuda
    Push "registro: $AuroraRegistro"
    Call AuroraScriviNuda
    aurora_apri_fine:
    Pop $6
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Pop $0
  FunctionEnd

  Function AuroraScriviFine
    Push $0
    Push $1
    Push $2
    Push $3
    Push $4
    Push $5
    Push $6
    ${GetTime} "" "L" $0 $1 $2 $3 $4 $5 $6
    Push "fine: $2-$1-$0 $4:$5:$6"
    Call AuroraScriviNuda
    Pop $6
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Pop $0
  FunctionEnd

  ; La cartella di destinazione e lo spazio libero, quando la cartella e' quella definitiva
  Function AuroraPrimaDellaCopia
    Push $0
    Push $1
    StrCpy $0 $INSTDIR 3
    ${DriveSpace} "$0" "/D=F /S=M" $1
    !insertmacro AuroraScrivi "cartella di installazione: $INSTDIR (spazio libero sul disco: $1 MB)"
    ; Aurora occupa circa 150 MB; sotto i 500 MB liberi la copia o Windows possono fallire
    ${If} $1 != ""
    ${AndIf} $1 < 500
      !insertmacro AuroraScrivi "AVVISO: poco spazio libero sul disco di destinazione ($1 MB)"
      StrCpy $AuroraPocoSpazio 1
    ${EndIf}
    Pop $1
    Pop $0
  FunctionEnd

  ; PowerShell e lo script del rapporto, estratto nella cartella temporanea dell'installatore. Senza PowerShell $AuroraScript resta vuoto
  Function AuroraPreparaScript
    ${If} $AuroraScript != ""
      Return
    ${EndIf}
    StrCpy $AuroraPowerShell "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe"
    IfFileExists $AuroraPowerShell 0 aurora_script_fine
    InitPluginsDir
    SetOverwrite on
    File "/oname=$PLUGINSDIR\aurora-rapporto.ps1" "${AURORA_INSTALLER_DIR}\rapporto-installazione.ps1"
    IfFileExists "$PLUGINSDIR\aurora-rapporto.ps1" 0 aurora_script_fine
    StrCpy $AuroraScript "$PLUGINSDIR\aurora-rapporto.ps1"
    aurora_script_fine:
  FunctionEnd

  ; L'elenco dei dettagli della finestra (la tecnica «DumpLog»): ogni riga in fondo al registro. Solo con le finestre: in silenzio
  ; l'elenco non c'e' e bastano i passi scritti nel registro.
  !define /ifndef LVM_GETITEMCOUNT 0x1004
  !define /ifndef LVM_GETITEMTEXTW 0x1073
  Function AuroraCopiaDettagli
    ${If} ${Silent}
    ${OrIf} $AuroraRegistro == ""
      Return
    ${EndIf}
    Push $0
    Push $1
    Push $2
    Push $3
    Push $4
    Push $5
    Push $6
    FindWindow $0 "#32770" "" $HWNDPARENT
    GetDlgItem $0 $0 1016
    StrCmp $0 0 aurora_dettagli_fine
    SendMessage $0 ${LVM_GETITEMCOUNT} 0 0 $6
    IntCmp $6 0 aurora_dettagli_fine aurora_dettagli_fine
    FileOpen $5 $AuroraRegistro a
    StrCmp $5 "" aurora_dettagli_fine
    FileSeek $5 0 END
    FileWriteUTF16LE $5 "--- dettagli dell'installazione ($6 righe dell'elenco della finestra) ---$\r$\n"
    System::Call "*(&t${NSIS_MAX_STRLEN})p.r3"
    System::Call "*(i, i, i, i, i, p, i, i, p) p (0, 0, 0, 0, 0, r3, ${NSIS_MAX_STRLEN}) .r1"
    StrCpy $2 0
    aurora_dettagli_riga:
      StrCmp $2 $6 aurora_dettagli_chiudi
      System::Call "user32::SendMessage(p r0, i ${LVM_GETITEMTEXTW}, p r2, p r1)"
      System::Call "*$3(&t${NSIS_MAX_STRLEN} .r4)"
      FileWriteUTF16LE $5 "$4$\r$\n"
      IntOp $2 $2 + 1
      Goto aurora_dettagli_riga
    aurora_dettagli_chiudi:
    FileWriteUTF16LE $5 "--- fine dei dettagli ---$\r$\n"
    System::Free $1
    System::Free $3
    FileClose $5
    aurora_dettagli_fine:
    Pop $6
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Pop $0
  FunctionEnd

  ; Dopo la copia: i file che servono ci sono? (un antivirus puo' togliere rigctld o le DLL di Hamlib appena scritte). Con degli avvisi
  ; si guarda anche l'antivirus (PowerShell, al massimo 20 s); senza avvisi no, per non allungare ogni aggiornamento.
  Function AuroraDopoLaCopia
    Push $0
    Push $1
    Push $2
    StrCpy $2 0
    !insertmacro AuroraScrivi "copia dei file finita"
    ${ForEach} $0 1 6 + 1
      ${Select} $0
        ${Case} 1
          StrCpy $1 "${MAINBINARYNAME}.exe"
        ${Case} 2
          StrCpy $1 "dist\index.html"
        ${Case} 3
          StrCpy $1 "hamlib\rigctld.exe"
        ${Case} 4
          StrCpy $1 "hamlib\libhamlib-4.dll"
        ${Case} 5
          StrCpy $1 "hamlib\rotctld.exe"
        ${Case} 6
          StrCpy $1 "cty\cty.dat"
      ${EndSelect}
      ${IfNot} ${FileExists} "$INSTDIR\$1"
        !insertmacro AuroraScrivi "AVVISO: file mancante dopo la copia: $1 (tolto dall'antivirus?)"
        StrCpy $2 1
      ${EndIf}
    ${Next}
    ${IfNot} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
      StrCpy $AuroraCausa "antivirus"
      StrCpy $AuroraMotivo "programma mancante dopo la copia (tolto dall'antivirus?)"
    ${EndIf}
    ${If} $2 = 1
      Call AuroraPreparaScript
      ${If} $AuroraScript != ""
        nsExec::ExecToStack /TIMEOUT=20000 `"$AuroraPowerShell" ${AURORA_PS_ARGS} -File "$AuroraScript" -Azione ambiente`
        Pop $0
        Pop $1
        ${If} $0 == 0
          !insertmacro AuroraScrivi "$1"
        ${Else}
          !insertmacro AuroraScrivi "antivirus: non letto (PowerShell: $0)"
        ${EndIf}
      ${EndIf}
    ${EndIf}
    Push "esito: riuscita"
    Call AuroraScriviNuda
    Call AuroraScriviFine
    DetailPrint "$(auroraLogSaved) $AuroraRegistro"
    Call AuroraCopiaDettagli
    Pop $2
    Pop $1
    Pop $0
  FunctionEnd

  ; L'installazione non e' riuscita (.onInstFailed, WebView2, chiusura dopo un errore, versione piu' vecchia in silenzio): errore e
  ; ambiente nel registro e, se i rapporti non sono spenti in Aurora, il rapporto anonimo. Una volta sola.
  Function AuroraFallita
    ${If} $AuroraFallitaFatta = 1
      Return
    ${EndIf}
    StrCpy $AuroraFallitaFatta 1
    Push $0
    Push $1
    ${If} $AuroraMotivo == ""
      ${If} $AuroraPasso == ""
        StrCpy $AuroraPasso "?"
      ${EndIf}
      StrCpy $AuroraMotivo "installazione fermata durante: $AuroraPasso"
    ${EndIf}
    Call AuroraApriRegistro
    !insertmacro AuroraScrivi "ERRORE: $AuroraMotivo"
    Push "esito: fallita"
    Call AuroraScriviNuda
    Call AuroraScriviFine
    Call AuroraCopiaDettagli
    ${If} $AuroraRegistro == ""
      Goto aurora_fallita_fine
    ${EndIf}
    Call AuroraPreparaScript
    ${If} $AuroraScript == ""
      Push "rapporto dell'installatore: non mandato (PowerShell assente)"
      Call AuroraScriviNuda
      Goto aurora_fallita_fine
    ${EndIf}
    DetailPrint "$(auroraReportSending)"
    ; ambiente (antivirus, programmi che tengono i file) e invio in una sola PowerShell, al massimo 60 s (sotto alcuni antivirus
    ; PowerShell lanciata da un installatore parte in decine di secondi): senza rete o con PowerShell bloccata non si resta appesi
    ; AURORA_PROVA_SERVIZIO (solo nelle prove, mai nell'installatore vero): un servizio locale al posto di quello dei rapporti
    !ifdef AURORA_PROVA_SERVIZIO
      !define /redef AURORA_SERVIZIO_PROVA `-Servizio "${AURORA_PROVA_SERVIZIO}"`
    !else
      !define /redef AURORA_SERVIZIO_PROVA ""
    !endif
    nsExec::ExecToStack /TIMEOUT=60000 `"$AuroraPowerShell" ${AURORA_PS_ARGS} -File "$AuroraScript" -Azione manda -Registro "$AuroraRegistro" -Versione "${VERSION}" -Motivo "$AuroraMotivo" -Cartella "$INSTDIR" ${AURORA_SERVIZIO_PROVA}`
    Pop $0
    Pop $1
    ${If} $0 == 0
      Push "rapporto dell'installatore: mandato ($1)"
      Call AuroraScriviNuda
      DetailPrint "$(auroraReportSent)"
      StrCpy $AuroraRapporto "mandato"
    ${ElseIf} $0 == 2
      Push "rapporto dell'installatore: non mandato, rapporti spenti in Aurora"
      Call AuroraScriviNuda
      StrCpy $AuroraRapporto "spenti"
    ${Else}
      Push "rapporto dell'installatore: non mandato ($0 $1)"
      Call AuroraScriviNuda
    ${EndIf}
    aurora_fallita_fine:
    DetailPrint "$(auroraLogSaved) $AuroraRegistro"
    Pop $1
    Pop $0
    Call AuroraSpiega
  FunctionEnd

  ; La schermata finale di un'installazione non riuscita, nella lingua dell'installatore: il motivo in parole semplici, cosa fare e se il
  ; rapporto anonimo e' partito; poi Riprova (riapre questo installatore), Apri il registro (con la pagina delle segnalazioni) o Chiudi.
  ; In silenzio e in modo passivo niente finestre: l'aggiornamento di Aurora riapre l'installatore normale se non e' andato a buon fine.
  Function AuroraSpiega
    ${If} ${Silent}
    ${OrIf} $PassiveMode = 1
      Return
    ${EndIf}
    Push $0
    Push $1
    ; la causa, se chi si e' fermato non l'ha detta: dal passo in corso
    ${If} $AuroraCausa == ""
      ${If} $AuroraPasso == "copia dei file"
        ${If} $AuroraPocoSpazio = 1
          StrCpy $AuroraCausa "spazio"
        ${Else}
          StrCpy $AuroraCausa "file"
        ${EndIf}
      ${ElseIf} $AuroraPasso == "chiusura di Aurora"
      ${OrIf} $AuroraPasso == "pulizia della cartella hamlib"
      ${OrIf} $AuroraPasso == "controllo dei programmi che tengono aurora-desktop.exe (Restart Manager)"
        StrCpy $AuroraCausa "file"
      ${ElseIf} $AuroraPasso == "WebView2"
        StrCpy $AuroraCausa "webview2"
      ${EndIf}
    ${EndIf}
    ${Select} $AuroraCausa
      ${Case} "webview2"
        StrCpy $0 "$(auroraFailWebview2)"
      ${Case} "versione"
        StrCpy $0 "$(auroraFailVersion)"
      ${Case} "file"
        StrCpy $0 "$(auroraFailFiles)"
      ${Case} "spazio"
        StrCpy $0 "$(auroraFailSpace)"
      ${Case} "antivirus"
        StrCpy $0 "$(auroraFailAntivirus)"
      ${CaseElse}
        StrCpy $0 "$(auroraFailOther)"
    ${EndSelect}
    ${If} $AuroraRapporto == "mandato"
      StrCpy $1 "$(auroraFailReportSent)"
    ${ElseIf} $AuroraRapporto == "spenti"
      StrCpy $1 "$(auroraFailReportOff)"
    ${Else}
      StrCpy $1 "$(auroraFailReportNot)"
    ${EndIf}
    DetailPrint " "
    DetailPrint "$(auroraFailTitle)"
    DetailPrint "$0"
    DetailPrint "$1"
    SetDetailsView show
    !insertmacro AuroraScrivi "schermata finale: causa '$AuroraCausa', rapporto '$AuroraRapporto'"
    MessageBox MB_ICONSTOP|MB_YESNOCANCEL|MB_DEFBUTTON1|MB_SETFOREGROUND "$(auroraFailTitle)$\r$\n$\r$\n$0$\r$\n$\r$\n$1$\r$\n$\r$\n$(auroraFailAsk)" IDYES aurora_spiega_riprova IDNO aurora_spiega_registro
    Goto aurora_spiega_fine
    aurora_spiega_riprova:
      Exec '"$EXEPATH"'
      Quit
    aurora_spiega_registro:
      ${If} $AuroraRegistro != ""
        ExecShell "open" "notepad.exe" '"$AuroraRegistro"'
      ${EndIf}
      ExecShell "open" "https://github.com/wifi75/Aurora/issues"
    aurora_spiega_fine:
    Pop $1
    Pop $0
  FunctionEnd
!macroend
