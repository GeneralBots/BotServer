

if consulta = "cpf" then
    talk "Qual seu CPF?"
    hear cpf
    talk "Aguarde alguns instantes que eu localizo seu cadastro..."
    row = find "Cadastro.xlsx", "CPF=" + cpf 
    if row != null then
        talk "Oi, " + row.Nome +  "Tudo bem? " 
        talk "Seu código de cliente é " + row.Cod
        talk "Vamos te enviar o pedido para seu endereço em: "  + row.Endereço
        send file "boleta.pdf", "Pague já e evite multas!"
    else
        talk "Tente novamente."
    end if
else
    talk "Qual seria seu código?"
    hear cod
    talk "Aguarde alguns instantes que eu localizo seu cadastro..."
    row = find "Cadastro.xlsx", "Cod=" + cod
    if row != null then
        talk "Oi, " + row.Nome +  "Tudo bem? " 
        talk "Seu CPF é " + row.CPF
        talk "Vamos te enviar o pedido para seu endereço em: "  + row.Endereço
        send file "boleta.pdf", "Pague já e evite multas!"
    else
        talk "Tente novamente."
    end if
end if
