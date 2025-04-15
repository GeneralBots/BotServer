export const Messages = {
  'en-US': {
    bot_created: address => `Your bot has been created and it's available at: ${address}`,
    new_password: newPassword => `Your new password is: **${newPassword}**.`,
    ok_get_information: 'OK, I will get some information.',
    ok_procceding_creation: 'I have all that I need to create your Bot. Wait a moment...',
    own_voucher:
      'Got a voucher?',
    please_use_code: code => `Please, answer the Bot with the code: ${code}.`,
    validation_enter_valid_botname: 'Please enter a valid Bot Name.',
    validation_enter_valid_voucher: 'Please enter a valid voucher code.',
    welcome:
      "Welcome and let's create your Bot.  Also visit: https://gb.pragmatismo.com.br/privacy.html to learn more about our privacy policy.",
    whats_botname: "What's the Bot name?",
    thanks_payment: 'Thanks for choosing paying for General Bots.',
    boleto_mail: 'Boleto will be e-mailed to you.'
  },
  'pt-BR': {
    bot_created: address =>
      `Em alguns minutos seu Bot estará disponível no endereço: ${address}. Você receberá por e-mail, a notificação da criação. Entraremos em contato em até 1(um) dia útil para realizar a configuração final.`,
    new_password: newPassword => `Sua nova senha é: **${newPassword}**.`,
    ok_get_information: 'OK, vou solicitar algumas informações.',
    ok_procceding_creation: 'Tenho tudo que preciso para criar seu Bot, só um instante...',
    own_voucher: 'Tem um voucher algo como GB2020 ou não?',
    please_use_code: code => `Por favor, responda ao bot com o código: ${code}.`,
    validation_enter_valid_botname:
      'Por favor, digite um nome de Bot válido, usando apenas letras maiúsculas e minúsculas sem espaços ou outros caracteres.',
    validation_enter_valid_voucher: 'Por favor, digite um código de voucher válido.',
    welcome:
      'Bem-vinda(o) e vamos criar seu bot.  Visite também: https://gb.pragmatismo.com.br/privacy.html para conhecer nossa política de privacidade',
    whats_botname: 'Qual é o nome do *Bot*? (Apenas letras maiúsculas e minúsculas, sem espaços ou demais caracteres)',
    thanks_payment: 'Gratidão por escolher o plano pago do General Bots.',
    boleto_mail: 'Boleto será enviado por e-mail.'
  }
};
