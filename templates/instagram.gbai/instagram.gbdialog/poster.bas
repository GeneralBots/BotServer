REM SET SCHEDULE "* 8 * * * *"
user = “user@domain.com”
pass= "*************"
o =get "https://oooooooooo"
caption = REWRITE "Crie um post sobre hotmart e seus produtos, no estilo dica do dia incluíndo 10 hashtags, estilo instagram o texto! Importante, retorne só a saída de texto pronta" 
image = GET IMAGE caption
POST username, password, image, caption
