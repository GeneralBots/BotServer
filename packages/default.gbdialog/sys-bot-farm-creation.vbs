' General Bots Copyright (c) Pragmatismo.io. All rights reserved.             
' Licensed under the AGPL-3.0. 

talk "Please, tell me what is the Bot name?"
hear name

talk "If you tell me your username/password, I can show service subscription list to you."
talk "What is your Username (eg.: human@domain.bot)"
hear email

talk "Your password? (Will be discarded after sigining process)"
talk "Let's generate a very dificult to guess password for the new bot:"
generate a password
talk "Your password is *" + password + "*. Keep it on a safe place only acessible to you."
talk "Can you describe in a few words what the bot is about?"
hear description

talk "Please, choose what subscription would you like to connect to:"
hear one of subscriptions (email, password) into subscriptionId

talk "Please, provide the cloud location just like 'westus'?"
hear cloudLocation 

talk "Please, provide the Authoring Key for NLP service (LUIS)?"
hear nlpKey

talk "Sorry, this part cannot be automated yet due to Microsoft schedule, please go to https://apps.dev.microsoft.com/portal/register-app to generate manually an App ID and App Secret."
wait 1
talk "Please, provide the App ID you just generated:"
hear appId

talk "Please, provide the Generated Password:"
hear appPassword

talk "Now, I am going to create a Bot farm... Wait 5 minutes or more..."
create bot farm (name, username, password, description, cloudLocation, nlpKey, appId, appPassword, subscriptionId)
