list = DIR "QCARobot.gbdrive"

FOR EACH item IN list
TALK "Verificando: " + item.name
DEBUG item
oldDays = DATEDIFF date, item.modified, "day"

IF oldDays > 3 THEN
TALK "O arquivo ${item.name} será arquivado, pois está expirado."
blob = UPLOAD item
TALK Upload para o Azure realizado.

SAVE "log.xlsx", "archived",today,now, item.path, item.name, item.size, item.modified, blob.md5
REM DELETE item
REM TALK Arquivo removido do SharePoint.
ELSE
TALK "O arquivo ${item.name} não precisa de arquivamento."
END IF
NEXT