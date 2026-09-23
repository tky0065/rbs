# L'enregistrement du README : de `rbs new` à la ressource que rend l'API.
#
# Ne se lance pas seul : `regenere.sh` pose l'environnement que lit `preparation.sh`, puis
# vérifie qu'aucune commande n'a échoué. Ce qui est caché (`Hide`) est coupé au montage,
# pas omis : l'amorce, le jeton et la capture tournent dans ce même terminal.

# Les images une à une, et non un GIF : l'assemblage de VHS 0.12 échoue sans un mot sous
# ffmpeg 9, et `regenere.sh` le fait lui-même, à la cadence de `Set Framerate`.
Output images/
Require docker
Require jq
Require make
Require node

Set Shell "bash"
Set FontSize 15
Set Width 1200
Set Height 720
Set Padding 24
Set Framerate 12
Set TypingSpeed 35ms
Set Theme "Catppuccin Mocha"
Set WaitTimeout 60s

Hide
Type "source $ENREG_ICI/preparation.sh && clear" Enter
Wait
Show

Type "rbs --version" Enter
Wait
Sleep 1.5s

Type "rbs new tickets --yes --lang en" Enter
Wait
Sleep 2.5s

Type "cd tickets" Enter
Wait
Hide
Type "git add -A && git commit -qm 'rbs new tickets' && clear" Enter
Wait
Show

Type "rbs add frontend-admin" Enter
Wait
Sleep 4s

Hide
Type "git add -A && git commit -qm 'rbs add frontend-admin' && clear" Enter
Wait
Show

Type "rbs generate crud tickets --fields 'title:string,body:text,priority:enum(low,normal,high),resolved:bool'" Enter
Wait
Sleep 4s

Hide
Type "bash $ENREG_ICI/amorce.sh > $ENREG_TRAVAIL/amorce.log 2>&1; clear" Enter
Wait@20m
Show

Type "make dev &" Enter
Wait+Screen@3m /ready in/
Wait+Screen@3m /127\.0\.0\.1:8080/
Sleep 1.5s

Hide
Type "TOKEN=$(bash $ENREG_ICI/jeton.sh) && echo jeton-$((1))-lu" Enter
Wait+Screen /jeton-1-lu/
Type "clear" Enter
Sleep 500ms
Show

Type `curl -s --fail-with-body localhost:8080/tickets -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' -d '{"title":"Printer on fire","body":"Third floor, again","priority":"high","resolved":false}' | jq` Enter
Wait+Screen /updated_at/
Sleep 6s

Hide
Type "bash $ENREG_ICI/capture.sh > $ENREG_TRAVAIL/capture.log 2>&1 && echo capture-$((1))-prise" Enter
Wait+Screen@2m /capture-1-prise/

Type "kill %1 && { wait || true; }" Enter
Wait@30s
