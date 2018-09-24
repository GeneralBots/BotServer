export const Messages = {
  "en-US": {
    authenticate: "Please, authenticate:",
    welcome: "Welcome to Pragmatismo.io GeneralBots Administration.",
    which_task: "Which task do you wanna run now?",
    working:(command)=> `I'm working on ${command}...`,
    finshed_working:"Done.",
    unknown_command: text =>
      `Well, but ${text} is not a administrative General Bots command, I will try to search for it.`,
    hi: text => `Hello, ${text}.`,
    undeployPackage: text => `Undeploying package ${text}...`,
    deployPackage: text => `Deploying package ${text}...`,
    redeployPackage: text => `Redeploying package ${text}...`,
    packageUndeployed: text => `Package ${text} undeployed...`,
    consent: (url)=>`Please, consent access to this app at: [Microsoft Online](${url}).`,
    wrong_password: "Sorry, wrong password. Please, try again."
  },
  "pt-BR": {
    show_video: "Vou te mostrar um vídeo. Por favor, aguarde...",
    hi: msg => `Oi, ${msg}.`
  }
};
