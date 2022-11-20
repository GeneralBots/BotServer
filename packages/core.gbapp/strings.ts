export const Messages = {
  global_quit: /^(\bsair\b|\bsai\b|\bchega\b|\bexit\b|\bquit\b|\bfinish\b|\bend\b|\bausfahrt\b|\bverlassen\b)/i,
  'en-US': {
    show_video: 'I will show you a video, please wait...',
    good_morning: 'good morning',
    good_evening: 'good evening',
    good_night: 'good night',
    hi: msg => `Hello, ${msg}.`,
    very_sorry_about_error: `I'm sorry to inform that there was an error which was recorded to be solved.`,
    canceled: 'Canceled. If I can be useful, let me know how',
    whats_email: "What's your E-mail address?",
    which_language: 'Please, type the language name you would like to talk through.',
    validation_enter_valid_email: 'Please enter a valid e-mail.',
    language_chosen: "Very good, so let's go...",
    affirmative_sentences: /^(\bsim\b|\bs\b|\bpositivo\b|\bafirmativo\b|\bclaro\b|\bevidente\b|\bsem dúvida\b|\bconfirmo\b|\bconfirmar\b|\bconfirmado\b|\buhum\b|\bsi\b|\by\b|\byes\b|\bsure\b)/i,
    will_answer_projector: "I'll answer on the projector to a better experience..."
  },
  'pt-BR': {
    show_video: 'Vou te mostrar um vídeo. Por favor, aguarde...',
    good_morning: 'bom dia',
    good_evening: 'boa tarde',
    good_night: 'boa noite',
    hi: msg => `Oi, ${msg}.`,
    very_sorry_about_error: `Lamento, ocorreu um erro que já foi registrado para ser tratado.`,
    canceled: 'Cancelado, avise como posso ser útil novamente.',
    whats_email: 'Qual seu e-mail?',
    which_language: 'Por favor, digite o idioma que você gostaria de usar para conversarmos.',
    validation_enter_valid_email: 'Por favor digite um email válido.',
    language_chosen: 'Muito bem, então vamos lá...',
    affirmative_sentences: /^(\bsim\b|\bs\b|\bpositivo\b|\bafirmativo\b|\bclaro\b|\bevidente\b|\bsem dúvida\b|\bconfirmo\b|\bconfirmar\b|\bconfirmado\b|\buhum\b|\bsi\b|\by\b|\byes\b|\bsure\b)/i,
    will_answer_projector: 'Vou te responder na tela para melhor visualização...'
  }
};
