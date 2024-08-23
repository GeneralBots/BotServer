lista = find "broadcast.csv"
indice = 1
do while indice < ubound(lista)
      linha = lista[indice]
      TALK TO linha.mobile, "Oi, " + linha.name + ".  Tudo bem? How about *General Bots* deployed? "
      wait 5
      save “Log.xlsx”, today, now, username, from,linha.mobile, linha.name
indice = indice + 1
loop
talk “O envio foi realizado. “
