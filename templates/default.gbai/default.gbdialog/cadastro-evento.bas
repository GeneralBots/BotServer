TALK "Olá, por favor, qual seu nome completo?"
HEAR nome
TALK "Qual o seu email para contato? "
HEAR email
TALK "Qual o seu telefone para contato? "
HEAR telefone
TALK "Função? "
HEAR funcao
TALK "Qual empresa que estará representando? " 
HEAR empresa
TALK "É freelancer?"
HEAR freelancer AS BOOLEAN
TALK "Você pretende participar do evento em qual cidade? " 
HEAR cidade AS "São Paulo", "Rio de Janeiro"
TALK "Você deseja receber outras informações do mailing da Quanta?"
SAVE "pessoas.xlsx", nome, email, telefone, funcao, empresa, freelancer