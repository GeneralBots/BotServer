REM SEND FILE “colegioinovar.PNG” 
SAVE “Log.xlsx”, from, mobile, today, now, "start"

TALK "Olá " + username + “, eu sou o *InovarBot*, uma inteligência artificial criada para facilitar o seu contato conosco (instituição de ensino) e auxiliar em seu crescimento/progresso/evolução educacional."

row = FIND “People.xlsx”, “mobile=” + from
IF row = null THEN

TALK Verifiquei que é seu primeiro contato conosco por aqui. Vamos fazer o seu cadastro e realizar a matrícula logo em seguida.
TALK Por favor, me informe o seu *Nome Completo*:
HEAR nome AS NAME
TALK Qual a sua *data de nascimento*? Exemplo: 23/09/2001.
HEAR datanasc AS DATE
TALK Informe por favor, um *e-mail* pra contato.
HEAR email as EMAIL
TALK Por favor, me informe o seu *CPF* (apenas números).
HEAR cpf AS INTEGER
TALK Qual o *número do seu RG ou CNH* (apenas números, por favor)?
HEAR rg AS INTEGER
TALK Qual o *Órgão emissor* do seu RG ou CNH?
HEAR orgaorg 
TALK Qual  a *data de emissão* do seu *RG* ou *CNH*? Exemplo: 15/08/2007
HEAR dataemite AS DATE
TALK Qual o seu endereço completo?
HEAR ender 
TALK Pronto! Agora vamos realizar a matrícula do aluno.\n\nPor favor, me informe o *Nome Completo*:
HEAR nomealuno AS NAME
TALK Me informe o *CPF* (apenas números) do aluno:
HEAR cpfaluno as INTEGER
TALK Qual a *data de nascimento* do aluno? Exemplo: 07/03/2010
HEAR datanascaluno AS DATE
TALK Qual o *RG* (apenas números) do aluno?
HEAR rgaluno AS DATE
TALK Qual o *Órgão Emissor* do documento?
HEAR orgaoaluno 
TALK Qual o *Data de Emissão* do documento?
HEAR emissaoaluno AS DATE
TALK Qual o nome do responsável financeiro do aluno?
HEAR respfinaluno AS NAME
TALK Vou registrar agora estes dados, um instante por favor...
SAVE People.xlsx, id, from, nome, datanasc, email, cpf, rg, orgaorg, dataemite, ender, nomealuno, cpfaluno, datanascaluno
TALK "Pronto,  + username + ! O cadastro foi realizado. Iremos entrar em contato. \n\nObrigado!"

ELSE
SAVE Log.xlsx, from, mobile, today, now, hello
TALK Olá,  + username + ! Bem-vinda(o) de volta. Você pode tirar dúvidas comigo sobre a secretaria.
END IF