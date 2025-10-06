let items  = FIND "gb.rob", "ACTION=EMUL"
FOR EACH item IN items

    PRINT item.company

    let website = item.website ?? ""
    if item.website == "" {
        website = WEBSITE OF item.company
        SET "gb.rob", "id="+ item.id, "website=" +  website
        PRINT website
    }

    let page = GET website
    let prompt = "Build the same simulator, keep js, svg, css, assets paths, just change title, keep six cases of six messages each (change and return VALID JSON with a minium of 6 cases and 6-8 messages each), but for " + item.company + " using just *content about the company* " + item.llm_notes + " from its website, so it is possible to create a good and useful emulator in the same langue as the content: " + page
    let alias = LLM "Return a single word for " + item.company + " like a token, no spaces, no special characters, no numbers, no uppercase letters."
    CREATE_SITE alias, "gb-emulator-base", prompt

    let to = item.emailcto
    let subject = "Simulador " + alias + " ficou pronto"
    let name = FIRST(item.contact)
    let body = "Oi, " + name + ". Tudo bem? Para vocês terem uma ideia do ambiente conversacional em AI e algumas possibilidades, preparamos o " + alias + " especificamente para vocês!"      + "\n\n Acesse o site: https://sites.pragmatismo.com.br/" + alias      + "\n\n" + "Para acessar o simulador, clique no link acima ou copie e cole no seu navegador."     + "\n\n" + "Para iniciar, escolha um dos casos conversacionais."     + "\n\n" + "Atenciosamente,\nRodrigo Rodriguez\n\n"

    let body = LLM "Melhora este e-mail: ------ " + body + " ----- mas mantem o link e inclui alguma referência ao histórico com o cliente: " + item.history

	CREATE_DRAFT to, subject, body

    SET "gb.rob", "id="+ item.id, "ACTION=CALL"
    SET "gb.rob", "id="+ item.id, "emulator=true"

    WAIT 3000
NEXT item
