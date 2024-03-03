/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

'use strict';

import { GBMinInstance } from 'botlib';
import OpenAI from "openai";
import { OpenAIChat } from 'langchain/llms/openai';
import { CallbackManager } from 'langchain/callbacks';
import { ChatPromptTemplate, HumanMessagePromptTemplate, SystemMessagePromptTemplate } from 'langchain/prompts';
import { LLMChain } from 'langchain/chains';
import { BufferWindowMemory } from 'langchain/memory';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import Path from 'path';
import * as Fs from 'fs';
import { HNSWLib } from 'langchain/vectorstores/hnswlib';
import { GuaribasSubject } from '../../kb.gbapp/models/index.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';

export class ChatServices {

  private static async getRelevantContext(
    vectorStore: HNSWLib,
    sanitizedQuestion: string,
    numDocuments: number
  ): Promise<string> {
    const documents = await vectorStore.similaritySearch(sanitizedQuestion, numDocuments);
    return documents
      .map((doc) => doc.pageContent)
      .join(', ')
      .trim()
      .replaceAll('\n', ' ');
  }


  public static async sendMessage(min: GBMinInstance, text: string) {
    let key;
    if (process.env.OPENAI_KEY) {
      key = process.env.OPENAI_KEY;
    }
    else {
      key = min.core.getParam(min.instance, 'Open AI Key', null);
    }

    if (!key) {
      throw new Error('Open AI Key not configured in .gbot.');
    }
    let functions = [];

    // Adds .gbdialog as functions if any to GPT Functions.

    await CollectionUtil.asyncForEach(Object.values(min.scriptMap), async script => {
      const path = DialogKeywords.getGBAIPath(min.botId, "gbdialog", null);
      const localFolder = Path.join('work', path, `${script}.json`);

      if (Fs.existsSync(localFolder)) {
        const func = Fs.readFileSync(localFolder).toJSON();
        functions.push(func);
      }

    });

    // Calls Model.

    const openai = new OpenAI({
      apiKey: key
    });
    const chatCompletion = await openai.chat.completions.create({
      model: "gpt-3.5-turbo",
      messages: [{ role: "user", content: text }],
      functions: functions
    });
    return chatCompletion.choices[0].message.content;
  }



  /**
   * Generate text
   *
   * CONTINUE keword.
   *
   * result = CONTINUE text
   *
   */
  public static async continue(min: GBMinInstance, text: string, chatId) {
    let key;
    if (process.env.OPENAI_KEY) {
      key = process.env.OPENAI_KEY;
    }
    else {
      key = min.core.getParam(min.instance, 'Open AI Key', null);
    }

    if (!key) {
      throw new Error('Open AI Key not configured in .gbot.');
    }
    // const openai = new OpenAI({
    //   apiKey: key
    // });
    // const chatCompletion = await openai.chat.completions.create({
    //   model: "gpt-3.5-turbo",
    //   messages: [{ role: "user", content: text }]

    // });
    // return chatCompletion.choices[0].message.content;
  }

  public static async answerByGPT(min: GBMinInstance,
    query: string,
    searchScore: number,
    subjects: GuaribasSubject[]
  ) {

    if (!process.env.OPENAI_KEY) {
      return { answer: undefined, questionId: 0 };
    }


    const contextVectorStore = min['vectorStore'];
    const question = query.trim().replaceAll('\n', ' ');
    const context = await this.getRelevantContext(contextVectorStore, question, 1);

    const systemPrompt = SystemMessagePromptTemplate.fromTemplate(
    `You are $${min.botId}`);

    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );


    const tools = ""// TODO: add func  list.
 
    const chatPrompt = ChatPromptTemplate.fromPromptMessages([
      systemPrompt,
      HumanMessagePromptTemplate.fromTemplate(`Answer in ${contentLocale}. 
      You have access to the context (RELEVANTDOCS) provided by the user.
      
      When answering think about whether the question in RELEVANTDOCS, but never mention
      to user about the source.
      Don’t justify your answers. Don't refer to yourself in any of the created content.
      Don´t prefix RESPONSE: when answering the user.
      RELEVANTDOCS: {context}

      QUESTION: """{input}"""

      You have the following tools that you can invoke based on the user inquiry. 
      Tools: 

        ${tools}

      `),
    ]);




    const windowMemory = new BufferWindowMemory({
      returnMessages: false,
      memoryKey: 'immediate_history',
      inputKey: 'input',
      k: 2,
    });

    const callbackManager = CallbackManager.fromHandlers({
      // This function is called when the LLM generates a new token (i.e., a prediction for the next word)
      async handleLLMNewToken(token: string) {
        
      },
    });
        
    const llm = new OpenAIChat({
      streaming: true,
      callbackManager,
      modelName: 'gpt-3.5-turbo',
    });
    
    const chain = new LLMChain({
      prompt: chatPrompt,
      memory: windowMemory,
      llm,
    });

    const response = await chain.call({
      input: question,
      context,
      history: '',
      immediate_history: '',
    });
    if (response) {

      return { answer: response.text, questionId: 0 };
    }

    return { answer: undefined, questionId: 0 };
  }


}
