value = get "list.xslx", "A1:A1"

set "list.xslx", "A1:A1", "value"

myVar = find "chamadosbug748.xlsx", "CHAMADO=" + "5521979047667-44129-10" 
status="alterado" 
set "chamadosbug748.xlsx", "E" + myVar.line + ":E" + myVar.line, status 
res = get "chamadosbug748.xlsx", "E" + myVar.line + ":E" + myVar.line 
talk "Obrigado e até a próxima e veja bem, o resultado é esse: " + res 
