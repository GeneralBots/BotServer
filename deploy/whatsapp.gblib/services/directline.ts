/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
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
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

const Path = require("path");
const Fs = require("fs");
const _ = require("lodash");
const Parse = require("csv-parse");
const Async = require("async");
const UrlJoin = require("url-join");
const Walk = require("fs-walk");
const logger = require("../../../src/logger");
const Swagger = require('swagger-client');
const open = require('open');
const rp = require('request-promise');
import { GBServiceCallback, GBService, IGBInstance } from "botlib";

export class WhatsappDirectLine extends GBService {

    pollInterval = 1000;
    directLineSecret = '';
    directLineClientName = 'DirectLineClient';
    directLineSpecUrl = 'https://docs.botframework.com/en-us/restapi/directline3/swagger.json';

    constructor(directLineSecret) {
        super();

        this.directLineSecret = directLineSecret;

        let directLineClient = rp(this.directLineSpecUrl)
            .then(function (spec) {
                return new Swagger({
                    spec: JSON.parse(spec.trim()),
                    usePromise: true
                });
            })
            .then(function (client) {
                client.clientAuthorizations.add('AuthorizationBotConnector',
                    new Swagger.ApiKeyAuthorization('Authorization', 'Bearer ' + this.directLineSecret, 'header'));
                return client;
            })
            .catch(function (err) {
                console.error('Error initializing DirectLine client', err);
            });

        directLineClient.then(function (client) {
            client.Conversations.Conversations_StartConversation()                          // create conversation
                .then(function (response) {
                    return response.obj.conversationId;
                })                            // obtain id
                .then(function (conversationId) {
                    this.sendMessagesFromConsole(client, conversationId);                        // start watching console input for sending new messages to bot
                    this.pollMessages(client, conversationId);                                   // start polling messages from bot
                })
                .catch(function (err) {
                    console.error('Error starting conversation', err);
                });
        });
    }

    sendMessagesFromConsole(client, conversationId) {
        var stdin = process.openStdin();
        process.stdout.write('Command> ');
        stdin.addListener('data', function (e) {
            var input = e.toString().trim();
            if (input) {
                // exit
                if (input.toLowerCase() === 'exit') {
                    return process.exit();
                }

                // send message
                client.Conversations.Conversations_PostActivity(
                    {
                        conversationId: conversationId,
                        activity: {
                            textFormat: 'plain',
                            text: input,
                            type: 'message',
                            from: {
                                id: this.directLineClientName,
                                name: this.directLineClientName
                            }
                        }
                    }).catch(function (err) {
                        console.error('Error sending message:', err);
                    });

                process.stdout.write('Command> ');
            }
        });
    }

    /** Poll Messages from conversation using DirectLine client */
    pollMessages(client, conversationId) {
        console.log('Starting polling message for conversationId: ' + conversationId);
        var watermark = null;
        setInterval(function () {
            client.Conversations.Conversations_GetActivities({ conversationId: conversationId, watermark: watermark })
                .then(function (response) {
                    watermark = response.obj.watermark;                                 // use watermark so subsequent requests skip old messages
                    return response.obj.activities;
                })
                .then(this.printMessages);
        }, this.pollInterval);
    }

    printMessages(activities) {
        if (activities && activities.length) {
            // ignore own messages
            activities = activities.filter(function (m) { return m.from.id !== this.directLineClientName });

            if (activities.length) {

                // print other messages
                activities.forEach(this.printMessage);

                process.stdout.write('Command> ');
            }
        }
    }

    printMessage(activity) {
        if (activity.text) {
            console.log(activity.text);
        }

        if (activity.attachments) {
            activity.attachments.forEach(function (attachment) {
                switch (attachment.contentType) {
                    case "application/vnd.microsoft.card.hero":
                        this.renderHeroCard(attachment);
                        break;

                    case "image/png":
                        console.log('Opening the requested image ' + attachment.contentUrl);
                        open(attachment.contentUrl);
                        break;
                }
            });
        }
    }

    renderHeroCard(attachment) {
        var width = 70;
        var contentLine = function (content) {
            return ' '.repeat((width - content.length) / 2) +
                content +
                ' '.repeat((width - content.length) / 2);
        }

        console.log('/' + '*'.repeat(width + 1));
        console.log('*' + contentLine(attachment.content.title) + '*');
        console.log('*' + ' '.repeat(width) + '*');
        console.log('*' + contentLine(attachment.content.text) + '*');
        console.log('*'.repeat(width + 1) + '/');
    }
}