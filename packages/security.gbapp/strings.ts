export const Messages = {
  'en-US': {
    whats_name: "What's your name?",
    whats_mobile: "What's your mobile number including country code (e.g. +1 222 9998888)?",
    confirm_mobile: 'Please type the code just sent to your mobile.',
    whats_email: "What's your E-mail address?",
    validation_enter_name: 'Please enter your full name.',
    validation_enter_valid_mobile: 'Please enter a valid mobile number.',
    validation_enter_valid_email: 'Please enter a valid e-mail.',
    authenticated: 'You are now authenticated.',
    not_authorized: 'Wrong verification code. Not authenticated yet. Try again, please.',
    please_use_code:(code)=> `Please, answer the Bot with the code: ${code}.`
    
  },
  'pt-BR': {
    whats_name: 'Qual o seu nome?',
    whats_email: 'Qual o seu e-mail?',
    whats_mobile: 'Qual o seu celular?',
    confirm_mobile: 'Por favor, digite o código enviado para seu celular.',
    confirm_mobile_again:
      `Esse não me parece ser um código numérico válido. Por favor, digite novamente o
      código enviado para seu celular.`,
    validation_enter_valid_email: 'Por favor, digite um e-mail válido no formato nome@domínio.com.br.',
    validation_enter_name: 'Por favor, digite seu nome completo',
    validation_enter_valid_mobile: 'Por favor, insira um número de celular válido (ex.: +55 21 98888-7766).',
    authenticated: 'Você está autenticada(o).',
    not_authorized: 'Código de identificação inválido. Não autorizado, tente novamente, por favor.',
    please_use_code:(code)=> `Por favor, responda ao bot com o código: ${code}.`

  }
};
