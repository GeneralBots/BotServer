TALK "For favor, digite a mensagem que deseja enviar:"
HEAR message

TALK "Analizando template ... (antes de mandar para a META)"
report = LLM "Esta mensagem vai ser aprovada pelo WhatsApp META como Template? Tem recomendação? Se estiver OK, responda o texto: OK. Do contrário, avalie o que deve ser feito."

IF report <> "OK" THEN
    TALK "A mensagem não será aprovada pela Meta. " + report
END IF

TALK "Envie agora o arquivo de imagem de cabefalho:"
HEAR plan AS FILE

TALK "É para um arquivo ou todos?"
HEAR in AS FILE

PUBLISH

IF in.isValid THEN
    list = FIND in.filename, "Perfil=" + grupos
ELSE
    list = GET "broadcast"
END IF

SET MAX LINES 2020

index = 1

DO WHILE index < UBOUND(list)
    row = list[index]

    SEND TEMPLATE TO row.telefone. filename

    WAIT 0.1

    index = index + 1

LOOP

TALK "OK, o envio foi realizado. Para saber mais, digite /report."
