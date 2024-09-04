TALK "Qual o número do processo? "
HEAR processo
text = GET "processo.pdf"
text = "Com base neste documento, responda as dúvidas da pessoa: \n\n" + text
SET CONTEXT text 
SET 
TALK "Processo ${processo} carregado. Pode me perguntar qualquer coisa do processo ou me peça um resumo da forma que você precisar.   "
