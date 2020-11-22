/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             |
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

/**
 * @fileoverview Conversation handling and external service calls.
 */

'use strict';

import { MessageFactory, RecognizerResult, TurnContext } from 'botbuilder';
import { LuisRecognizer } from 'botbuilder-ai';
import { GBDialogStep, GBLog, GBMinInstance, IGBCoreService } from 'botlib';
import { GBServer } from '../../../src/app';
import { Readable } from 'stream';
import { GBAdminService } from '../../admin.gbapp/services/GBAdminService';
import { SecService } from '../../security.gbapp/services/SecService';
import { AnalyticsService } from '../../analytics.gblib/services/AnalyticsService';
import { MicrosoftAppCredentials } from 'botframework-connector';
import { GBConfigService } from './GBConfigService';
const urlJoin = require('url-join');
const PasswordGenerator = require('strict-password-generator').default;
const Nexmo = require('nexmo');
import { CollectionUtil, AzureText } from 'pragmatismo-io-framework';
const { join } = require('path');
const shell = require('any-shell-escape');
const { exec } = require('child_process');
const prism = require('prism-media');
const uuidv4 = require('uuid/v4');
const request = require('request-promise-native');
const fs = require('fs');
const SpeechToTextV1 = require('ibm-watson/speech-to-text/v1');
const { IamAuthenticator } = require('ibm-watson/auth');

export interface LanguagePickerSettings {
  defaultLocale?: string;
  supportedLocales?: string[];
}

/**
 * Provides basic services for handling messages and dispatching to back-end
 * services like NLP or Search.
 */
export class GBConversationalService {
  public coreService: IGBCoreService;

  constructor(coreService: IGBCoreService) {
    this.coreService = coreService;
  }

  public getNewMobileCode() {
    const passwordGenerator = new PasswordGenerator();
    const options = {
      upperCaseAlpha: false,
      lowerCaseAlpha: false,
      number: true,
      specialCharacter: false,
      minimumLength: 4,
      maximumLength: 4
    };
    let code = passwordGenerator.generatePassword(options);
    return code;
  }

  public getCurrentLanguage(step: GBDialogStep) {
    return step.context.activity.locale;
  }

  public async sendFile(
    min: GBMinInstance,
    step: GBDialogStep,
    mobile: string,
    url: string,
    caption: string
  ): Promise<any> {
    if (step !== null) {
      if (!isNaN(step.context.activity.from.id as any)) {
        mobile = step.context.activity.from.id;
        GBLog.info(`Sending file ${url} to ${mobile}...`);
        const filename = url.substring(url.lastIndexOf('/') + 1);
        await min.whatsAppDirectLine.sendFileToDevice(mobile, url, filename, caption);
      } else {
        GBLog.info(
          `Sending ${url} as file attachment not available in this channel ${step.context.activity.from.id}...`
        );
        await min.conversationalService.sendText(min, step, url);
      }
    } else {
      GBLog.info(`Sending file ${url} to ${mobile}...`);
      const filename = url.substring(url.lastIndexOf('/') + 1);
      await min.whatsAppDirectLine.sendFileToDevice(mobile, url, filename, caption);
    }
  }

  public async sendAudio(min: GBMinInstance, step: GBDialogStep, url: string): Promise<any> {
    const mobile = step.context.activity.from.id;
    await min.whatsAppDirectLine.sendAudioToDevice(mobile, url);
  }

  public async sendEvent(min: GBMinInstance, step: GBDialogStep, name: string, value: Object): Promise<any> {
    if (step.context.activity.channelId === 'webchat') {
      const msg = MessageFactory.text('');
      msg.value = value;
      msg.type = 'event';
      msg.name = name;

      return await step.context.sendActivity(msg);
    }
  }

  // tslint:disable:no-unsafe-any due to Nexmo.
  public async sendSms(min: GBMinInstance, mobile: string, text: string): Promise<any> {
    return new Promise((resolve: any, reject: any): any => {
      const nexmo = new Nexmo({
        apiKey: min.instance.smsKey,
        apiSecret: min.instance.smsSecret
      });
      // tslint:disable-next-line:no-unsafe-any
      nexmo.message.sendSms(min.instance.smsServiceNumber, mobile, text, (err, data) => {
        if (err) {
          reject(err);
        } else {
          resolve(data);
        }
      });
    });
  }

  public async sendToMobile(min: GBMinInstance, mobile: string, message: string) {
    min.whatsAppDirectLine.sendToDevice(mobile, message);
  }

  public static async getAudioBufferFromText(speechKey, cloudRegion, text, locale): Promise<string> {
    return new Promise<string>(async (resolve, reject) => {
      const name = GBAdminService.getRndReadableIdentifier();

      const waveFilename = `work/tmp${name}.pcm`;
      const sdk = require('microsoft-cognitiveservices-speech-sdk');
      sdk.Recognizer.enableTelemetry(false);

      var audioConfig = sdk.AudioConfig.fromAudioFileOutput(waveFilename);
      var speechConfig = sdk.SpeechConfig.fromSubscription(speechKey, cloudRegion);

      var synthesizer = new sdk.SpeechSynthesizer(speechConfig, audioConfig);

      try {
        speechConfig.speechSynthesisLanguage = locale;
        speechConfig.speechSynthesisVoiceName = 'pt-BR-FranciscaNeural';

        synthesizer.speakTextAsync(text, result => {
          if (result.reason === sdk.ResultReason.SynthesizingAudioCompleted) {
            let raw = Buffer.from(result.audioData);
            fs.writeFileSync(waveFilename, raw);
            GBLog.info(`Audio data byte size: ${result.audioData.byteLength}.`);
            const oggFilenameOnly = `tmp${name}.ogg`;
            const oggFilename = `work/${oggFilenameOnly}`;

            const output = fs.createWriteStream(oggFilename);
            const transcoder = new prism.FFmpeg({
              args: ['-analyzeduration', '0', '-loglevel', '0', '-f', 'opus', '-ar', '16000', '-ac', '1']
            });

            fs.createReadStream(waveFilename).pipe(transcoder).pipe(output);

            let url = urlJoin(GBServer.globals.publicAddress, 'audios', oggFilenameOnly);
            resolve(url);
          } else {
            const error = 'Speech synthesis canceled, ' + result.errorDetails;
            reject(error);
          }
          synthesizer.close();
          synthesizer = undefined;
        });
      } catch (error) {
        reject(error);
      }
    });
  }

  public static async getTextFromAudioBuffer(speechKey, cloudRegion, buffer, locale): Promise<string> {
    return new Promise<string>(async (resolve, reject) => {
      try {
        const oggFile = new Readable();
        oggFile._read = () => {}; // _read is required but you can noop it
        oggFile.push(buffer);
        oggFile.push(null);

        const name = GBAdminService.getRndReadableIdentifier();

        const dest = `work/tmp${name}.wav`;
        const src = `work/tmp${name}.ogg`;
        fs.writeFileSync(src, oggFile.read());

        const makeMp3 = shell([
          'node_modules/ffmpeg-static/ffmpeg.exe',
          '-y',
          '-v',
          'error',
          '-i',
          join(process.cwd(), src),
          '-ar',
          '44100',
          '-ac',
          '1',
          '-acodec',
          'pcm_s16le',
          join(process.cwd(), dest)
        ]);

        exec(makeMp3, error => {
          if (error) {
            GBLog.error(error);
            return Promise.reject(error);
          } else {
            let data = fs.readFileSync(dest);

            const speechToText = new SpeechToTextV1({
              authenticator: new IamAuthenticator({ apikey: process.env.WATSON_STT_KEY }),
              url: process.env.WATSON_STT_URL
            });

            const params = {
              audio: data,
              contentType: 'audio/l16; rate=44100',
              model: 'pt-BR_BroadbandModel'
            };

            speechToText
              .recognize(params)
              .then(response => {
                if (response.result.results.length > 0) {
                  resolve(response.result.results[0].alternatives[0].transcript);
                }
              })
              .catch(error => {
                GBLog.error(error);
                return Promise.reject(error);
              });

            // let pushStream = sdk.AudioInputStream.createPushStream();
            // pushStream.write(data);
            // pushStream.close();

            // let audioConfig = sdk.AudioConfig.fromStreamInput(pushStream);
            // let speechConfig = sdk.SpeechConfig.fromSubscription(subscriptionKey, serviceRegion);
            // speechConfig.speechRecognitionLanguage = locale;
            // let recognizer = new sdk.SpeechRecognizer(speechConfig, audioConfig);

            // recognizer.recognizeOnceAsync(
            //   (result) => {

            //     resolve(result.text ? result.text : 'Speech to Text failed: Audio not converted');

            //     recognizer.close();
            //     recognizer = undefined;
            //   },
            //   (err) => {
            //     reject(err);

            //     recognizer.close();
            //     recognizer = undefined;
            //   });
          }
        });
      } catch (error) {
        GBLog.error(error);
        return Promise.reject(error);
      }
    });
  }

  // tslint:enable:no-unsafe-any

  public async sendMarkdownToMobile(min: GBMinInstance, step: GBDialogStep, mobile: string, text: string) {
    let sleep = ms => {
      return new Promise(resolve => {
        setTimeout(resolve, ms);
      });
    };
    enum State {
      InText,
      InImage,
      InImageBegin,
      InImageCaption,
      InImageAddressBegin,
      InImageAddressBody,
      InEmbedBegin,
      InEmbedEnd,
      InEmbedAddressBegin,
      InEmbedAddressEnd,
      InLineBreak,
      InLineBreak1,
      InLineBreak2
    }
    let state = State.InText;
    let currentImage = '';
    let currentText = '';
    let currentCaption = '';
    let currentEmbedUrl = '';

    //![General Bots](/instance/images/gb.png)
    for (let i = 0; i < text.length; i++) {
      const c = text.charAt(i);

      switch (state) {
        case State.InText:
          if (c === '!') {
            state = State.InImageBegin;
          } else if (c === '[') {
            state = State.InEmbedBegin;
          } else if (c === '\n') {
            state = State.InLineBreak;
          } else {
            state = State.InText;
            currentText = currentText.concat(c);
          }
          break;
        case State.InLineBreak:
          if (c === '\n') {
            state = State.InLineBreak1;
          } else if (c === '!') {
            state = State.InImageBegin;
          } else if (c === '[') {
            state = State.InEmbedBegin;
          } else {
            currentText = currentText.concat('\n', c);
            state = State.InText;
          }
          break;
        case State.InLineBreak1:
          if (c === '\n') {
            if (mobile === null) {
              await step.context.sendActivity(currentText);
            } else {
              await this.sendToMobile(min, mobile, currentText);
            }
            await sleep(3000);
            currentText = '';
            state = State.InText;
          } else if (c === '!') {
            state = State.InImageBegin;
          } else if (c === '[') {
            state = State.InEmbedBegin;
          } else {
            currentText = currentText.concat('\n', '\n', c);
            state = State.InText;
          }
          break;
        case State.InEmbedBegin:
          if (c === '=') {
            if (currentText !== '') {
              if (mobile === null) {
                await step.context.sendActivity(currentText);
              } else {
                await this.sendToMobile(min, mobile, currentText);
              }
              await sleep(3000);
            }
            currentText = '';
            state = State.InEmbedAddressBegin;
          }

          break;
        case State.InEmbedAddressBegin:
          if (c === ']') {
            state = State.InEmbedEnd;
            let url = currentEmbedUrl.startsWith('http')
              ? currentEmbedUrl
              : urlJoin(GBServer.globals.publicAddress, currentEmbedUrl);
            await this.sendFile(min, step, mobile, url, null);
            await sleep(5000);
            currentEmbedUrl = '';
          } else {
            currentEmbedUrl = currentEmbedUrl.concat(c);
          }
          break;
        case State.InEmbedEnd:
          if (c === ']') {
            state = State.InText;
          }
          break;
        case State.InImageBegin:
          if (c === '[') {
            if (currentText !== '') {
              if (mobile === null) {
                await step.context.sendActivity(currentText);
              } else {
                await this.sendToMobile(min, mobile, currentText);
              }
              await sleep(2900);
            }
            currentText = '';
            state = State.InImageCaption;
          } else {
            state = State.InText;
            currentText = currentText.concat('!').concat(c);
          }
          break;
        case State.InImageCaption:
          if (c === ']') {
            state = State.InImageAddressBegin;
          } else {
            currentCaption = currentCaption.concat(c);
          }
          break;
        case State.InImageAddressBegin:
          if (c === '(') {
            state = State.InImageAddressBody;
          }
          break;
        case State.InImageAddressBody:
          if (c === ')') {
            state = State.InText;
            let url = currentImage.startsWith('http')
              ? currentImage
              : urlJoin(GBServer.globals.publicAddress, currentImage);
            await this.sendFile(min, step, mobile, url, currentCaption);
            currentCaption = '';
            await sleep(4500);
            currentImage = '';
          } else {
            currentImage = currentImage.concat(c);
          }
          break;
      }
    }
    if (currentText !== '') {
      if (mobile === null) {
        await step.context.sendActivity(currentText);
      } else {
        await this.sendToMobile(min, mobile, currentText);
      }
    }
  }

  public async routeNLP(step: GBDialogStep, min: GBMinInstance, text: string): Promise<boolean> {
    if (min.instance.nlpAppId === null || min.instance.nlpAppId === undefined) {
      return false;
    }

    const model = new LuisRecognizer({
      applicationId: min.instance.nlpAppId,
      endpointKey: min.instance.nlpKey,
      endpoint: min.instance.nlpEndpoint
    });

    let nlp: RecognizerResult;
    try {
      const saved = step.context.activity.text;
      step.context.activity.text = text;
      nlp = await model.recognize(step.context);
      step.context.activity.text = saved;
    } catch (error) {
      // tslint:disable:no-unsafe-any
      if (error.statusCode === 404) {
        GBLog.warn('NLP application still not publish and there are no other options for answering.');

        return false;
      } else {
        const msg = `Error calling NLP, check if you have a published model and assigned keys. Error: ${
          error.statusCode ? error.statusCode : ''
        } {error.message; }`;

        throw new Error(msg);
      }
      // tslint:enable:no-unsafe-any
    }

    let nlpActive = false;
    let score = 0;

    Object.keys(nlp.intents).forEach(name => {
      score = nlp.intents[name].score;
      if (score > min.instance.nlpScore) {
        nlpActive = true;
      }
    });

    // Resolves intents returned from LUIS.

    const topIntent = LuisRecognizer.topIntent(nlp);
    if (topIntent !== undefined && nlpActive) {
      const intent = topIntent;
      if (intent === 'None') {
        return false;
      }

      GBLog.info(
        `NLP called: ${intent}, entities: ${nlp.entities.length}, score: ${score} > required (nlpScore): ${min.instance.nlpScore}`
      );

      try {
        step.activeDialog.state.options.entities = nlp.entities;
        await step.replaceDialog(`/${intent}`, step.activeDialog.state.options);

        return true;
      } catch (error) {
        const msg = `Error finding dialog associated to NLP event: ${intent}: ${error.message}`;

        throw new Error(msg);
      }
    }

    return false;
  }

  public async getLanguage(min: GBMinInstance, text: string): Promise<string> {
    return await AzureText.getLocale(
      min.instance.textAnalyticsKey ? min.instance.textAnalyticsKey : min.instance.textAnalyticsKey,
      min.instance.textAnalyticsEndpoint ? min.instance.textAnalyticsEndpoint : min.instance.textAnalyticsEndpoint,
      text
    );
  }

  public async spellCheck(min: GBMinInstance, text: string): Promise<string> {
    const key = min.instance.spellcheckerKey ? min.instance.spellcheckerKey : min.instance.spellcheckerKey;
    if (key) {
      text = text.charAt(0).toUpperCase() + text.slice(1);
      const data = await AzureText.getSpelledText(min.instance.spellcheckerKey, text);
      if (data !== text) {
        GBLog.info(`Spelling corrected (processMessageActivity): ${data}`);
        text = data;
      }
    }

    return text;
  }

  public async translate(min: GBMinInstance, text: string, language: string): Promise<string> {
    const translatorEnabled = () => {
      if (min.instance.params) {
        const params = JSON.parse(min.instance.params);
        return params ? params['Enable Worldwide Translator'] === 'TRUE' : false;
      }
      return false;
    };
    const endPoint = min.instance.translatorEndpoint;
    const key = min.instance.translatorKey;

    if (endPoint === null || !translatorEnabled() || process.env.TRANSLATOR_DISABLED === 'true') {
      return text;
    }

    if (text.length > 5000) {
      text = text.substr(0, 4999);
      GBLog.warn(`Text that bot will translate will be truncated due to MSFT service limitations.`);
    }

    let options = {
      method: 'POST',
      baseUrl: endPoint,
      url: 'translate',
      qs: {
        'api-version': '3.0',
        to: [language]
      },
      headers: {
        'Ocp-Apim-Subscription-Key': key,
        'Ocp-Apim-Subscription-Region': 'westeurope',
        'Content-type': 'application/json',
        'X-ClientTraceId': uuidv4().toString()
      },
      body: [
        {
          text: text
        }
      ],
      json: true
    };

    try {
      const results = await request(options);

      return results[0].translations[0].text;
    } catch (error) {
      const msg = `Error calling Translator service layer. Error is: ${error}.`;

      return Promise.reject(new Error(msg));
    }
  }

  public async prompt(min: GBMinInstance, step: GBDialogStep, text: string) {
    const user = await min.userProfile.get(step.context, {});
    const systemUser = user.systemUser;

    if (text !== null) {
      text = await min.conversationalService.translate(
        min,
        text,
        systemUser.locale
          ? systemUser.locale
          : min.core.getParam<string>(min.instance, 'Locale', GBConfigService.get('LOCALE'))
      );
      GBLog.info(`Translated text(prompt): ${text}.`);
    }

    return await step.prompt('textPrompt', text ? text : {});
  }

  public async sendText(min: GBMinInstance, step, text) {
    const member = step.context.activity.from;
    const user = await min.userProfile.get(step.context, {});
    const systemUser = user.systemUser;

    text = await min.conversationalService.translate(
      min,
      text,
      systemUser.locale
        ? systemUser.locale
        : min.core.getParam<string>(min.instance, 'Locale', GBConfigService.get('LOCALE'))
    );
    GBLog.info(`Translated text(sendText): ${text}.`);

    const analytics = new AnalyticsService();
    
    analytics.createMessage(min.instance.instanceId, user.conversation, null, text);

    if (!isNaN(member.id)) {
      await min.whatsAppDirectLine.sendToDevice(member.id, text);
    } else {
      await step.context.sendActivity(text);
    }
  }

  public async broadcast(min: GBMinInstance, message: string) {
    GBLog.info(`Sending broadcast notifications...`);

    let sleep = ms => {
      return new Promise(resolve => {
        setTimeout(resolve, ms);
      });
    };

    const service = new SecService();
    const users = await service.getAllUsers(min.instance.instanceId);
    await CollectionUtil.asyncForEach(users, async user => {
      if (user.conversationReference) {
        const ref = JSON.parse(user.conversationReference);
        MicrosoftAppCredentials.trustServiceUrl(ref.serviceUrl);
        await min.bot['createConversation'](ref, async t1 => {
          const ref2 = TurnContext.getConversationReference(t1.activity);
          await min.bot.continueConversation(ref2, async t2 => {
            await t2.sendActivity(message);
          });
        });
      } else {
        GBLog.info(`User: ${user.systemUserId} with no conversation ID while broadcasting.`);
      }
    });
  }

  public static kmpSearch(pattern, text) {
    pattern = pattern.toLowerCase();
    text = text.toLowerCase();
    if (pattern.length == 0)
      return 0; // Immediate match

    // Compute longest suffix-prefix table
    var lsp = [0]; // Base case
    for (var i = 1; i < pattern.length; i++) {
      var j = lsp[i - 1]; // Start by assuming we're extending the previous LSP
      while (j > 0 && pattern.charAt(i) != pattern.charAt(j))
        j = lsp[j - 1];
      if (pattern.charAt(i) == pattern.charAt(j))
        j++;
      lsp.push(j);
    }

    // Walk through text string
    var j = 0; // Number of chars matched in pattern
    for (var i = 0; i < text.length; i++) {
      while (j > 0 && text.charAt(i) != pattern.charAt(j))
        j = lsp[j - 1]; // Fall back in the pattern
      if (text.charAt(i) == pattern.charAt(j)) {
        j++; // Next char matched, increment position
        if (j == pattern.length)
          return i - (j - 1);
      }
    }
    return -1; // Not found
  }

}
