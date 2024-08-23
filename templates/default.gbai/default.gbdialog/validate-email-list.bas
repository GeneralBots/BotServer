SET MAX LINES 1000
list = FIND "Mailing Global.xlsx"
indice = 1
do while indice < ubound(list)
      	row = list[indice]
valid = IS VALID row.email
indice = indice + 1
IF valid THEN
Set "Mailing Global.xlsx", "B" + indice , "x"
END IF
loop
talk “Validate OK. “
