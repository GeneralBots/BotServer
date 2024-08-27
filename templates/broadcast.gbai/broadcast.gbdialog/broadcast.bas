list = FIND "broadcast.csv"
index = 1
DO WHILE index < UBOUND(list)
      row = list[index]
      TALK TO row.mobile, "Hi, " + row.name + ". How are you? How about *General Bots* deployed?"
      WAIT 5
      SAVE "Log.xlsx", TODAY, NOW, USERNAME, FROM, row.mobile, row.name
      index = index + 1
LOOP
TALK "The broadcast has been sent."
