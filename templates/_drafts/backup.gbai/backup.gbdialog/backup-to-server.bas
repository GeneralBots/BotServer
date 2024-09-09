list = DIR "default.gbdrive"

FOR EACH item IN list
    TALK "Checking: " + item.name
    oldDays = DATEDIFF date, item.modified, "day"

    IF oldDays > 3 THEN
        TALK "The file ${item.name} will be archived as it is expired."
        blob = UPLOAD item
        TALK "Upload to server completed."

        SAVE "log.xlsx", "archived", today, now, item.path, item.name, item.size, item.modified, blob.md5
        DELETE item
        TALK "File removed from storage."
    ELSE
        TALK "The file ${item.name} does not need to be archived."
    END IF
NEXT
