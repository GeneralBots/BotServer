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
const rp = require('request-promise');
import * as request from "request-promise-native";

import { GBServiceCallback, GBService, IGBInstance } from "botlib";

export class WhatsappDirectLine extends GBService {

    pollInterval = 1000;
    directLineSecret = '';
    directLineClientName = 'DirectLineClient';
    directLineSpecUrl = 'https://docs.botframework.com/en-us/restapi/directline3/swagger.json';

    constructor(directLineSecret) {
        super();

        this.directLineSecret = directLineSecret;


        // TODO: Migrate to Swagger 3.
        let directLineClient = rp(this.directLineSpecUrl)
            .then(function (spec) {
                return new Swagger({
                    spec: JSON.parse(spec.trim()),
                    usePromise: true
                });
            })
            .then(function (client) {
                client.clientAuthorizations.add('AuthorizationBotConnector',
                    new Swagger.ApiKeyAuthorization('Authorization', 'Bearer ' + directLineSecret, 'header'));
                return client;
            })
            .catch(function (err) {
                console.error('Error initializing DirectLine client', err);
            });

        // TODO: Remove *this* issue.
        let _this = this;
        directLineClient.then(function (client) {
            client.Conversations.Conversations_StartConversation()
                .then(function (response) {
                    return response.obj.conversationId;
                })                           
                .then(function (conversationId) {
                    _this.sendMessagesFromConsole(client, conversationId);
                    _this.pollMessages(client, conversationId);           
                })
                .catch(function (err) {
                    console.error('Error starting conversation', err);
                });
        });
    }

    sendMessagesFromConsole(client, conversationId) {
        let _this = this;
        var stdin = process.openStdin();
        process.stdout.write('Command> ');
        stdin.addListener('data', function (e) {
            var input = e.toString().trim();
            if (input) {
                // exit
                if (input.toLowerCase() === 'exit') {
                    return process.exit();
                }

                client.Conversations.Conversations_PostActivity(
                    {
                        conversationId: conversationId,
                        activity: {
                            textFormat: 'plain',
                            text: input,
                            type: 'message',
                            from: {
                                id: _this.directLineClientName,
                                name: _this.directLineClientName
                            }
                        }
                    }).catch(function (err) {
                        console.error('Error sending message:', err);
                    });

                process.stdout.write('Command> ');
            }
        });
    }

    /** TBD: Poll Messages from conversation using DirectLine client */
    pollMessages(client, conversationId) {
        let _this = this;
        console.log('Starting polling message for conversationId: ' + conversationId);
        var watermark = null;
        setInterval(function () {
            client.Conversations.Conversations_GetActivities({ conversationId: conversationId, watermark: watermark })
                .then(function (response) {
                    watermark = response.obj.watermark;                                 // use watermark so subsequent requests skip old messages
                    return response.obj.activities;
                })
                .then(_this.printMessages, _this.directLineClientName);
        }, this.pollInterval);
    }

    printMessages(activities, directLineClientName) {

        if (activities && activities.length) {
            // ignore own messages
            activities = activities.filter(function (m) { return m.from.id !== directLineClientName });

            if (activities.length) {

                // print other messages
                activities.forEach(activity => {
                    console.log(activity.text);
                });

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

    
    async sendToDevice(senderID, msg) {
        var options = {
            method: 'POST',
            url: 'https://www.waboxapp.com/api/send/chat',
            qs:
                {
                    token: '',
                    uid: '55****388**',
                    to: senderID,
                    custom_uid: 'GBZAP' + (new Date()).toISOString,
                    text: msg
                },
            headers:
                {
                    'cache-control': 'no-cache'
                }
        };

        const result = await request.get(options);

    }


}