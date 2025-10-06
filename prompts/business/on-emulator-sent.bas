let items  = FIND "gb.rob", "ACTION=EMUL_ASK"
FOR EACH item IN items

    let to = item.emailcto
    let subject = "Sobre o Simulador de AI enviado"
    let name = FIRST(item.contact)
    let body = GET "/EMUL-message.html"

	CREATE_DRAFT to, subject, body
    SET "gb.rob", "id="+ item.id, "ACTION=EMUL_ASKED"

NEXT item
