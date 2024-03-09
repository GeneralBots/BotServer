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
import { CallbackManager } from 'langchain/callbacks';
import { ChatPromptTemplate, HumanMessagePromptTemplate, SystemMessagePromptTemplate } from 'langchain/prompts';
import { ConversationChain, LLMChain } from 'langchain/chains';
import { BufferWindowMemory } from 'langchain/memory';
import { CollectionUtil } from 'pragmatismo-io-framework';
import { DialogKeywords } from '../../basic.gblib/services/DialogKeywords.js';
import Path from 'path';
import * as Fs from 'fs';
import { HNSWLib } from 'langchain/vectorstores/hnswlib';
import { GuaribasSubject } from '../../kb.gbapp/models/index.js';
import { GBConfigService } from '../../core.gbapp/services/GBConfigService.js';
import { ChatOpenAI } from "@langchain/openai";
import { JsonOutputFunctionsParser } from 'langchain/dist/output_parsers/openai_functions.js';
import { GBVMService } from '../../basic.gblib/services/GBVMService.js';


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

  /**
   * Generate text
   *
   * CONTINUE keword.
   *
   * result = CONTINUE text
   *
   */
  public static async continue(min: GBMinInstance, text: string, chatId) {

  }


  public static async answerByGPT(min: GBMinInstance, pid,
    query: string,
    searchScore: number,
    subjects: GuaribasSubject[]
  ) {

    return { answer: undefined, questionId: 0 };

    if (!process.env.OPENAI_API_KEY) {
      return { answer: undefined, questionId: 0 };
    }

    const systemPrompt = SystemMessagePromptTemplate.fromTemplate(
      `You are $${min.botId}`);

    const contentLocale = min.core.getParam(
      min.instance,
      'Default Content Language',
      GBConfigService.get('DEFAULT_CONTENT_LANGUAGE')
    );


    let functions = [];

    // Adds .gbdialog as functions if any to GPT Functions.

    await CollectionUtil.asyncForEach(Object.keys(min.scriptMap), async script => {
      const path = DialogKeywords.getGBAIPath(min.botId, "gbdialog", null);
      const functionJSON = Path.join('work', path, `${script}.json`);

      if (Fs.existsSync(functionJSON)) {
        const func = JSON.parse(Fs.readFileSync(functionJSON, 'utf8'));
        
        functions.push(func);
      }

    });


    let SystemPromptTailored = ''; // TODO: Load from user context.

    // Generates function definition for each function
    // in plain text to be used in system prompt.

    let functionDef = Object.keys(functions)
      .map((toolname) => `${functions[toolname].function.name}: ${functions[toolname].function.description}`)
      .join("\n");

    let promptTemplate = `Answer in ${contentLocale}. 
      You have access to the context (RELEVANTDOCS) provided by the user.
      
      When answering think about whether the question in RELEVANTDOCS, but never mention
      to user about the source.
      Don’t justify your answers. Don't refer to yourself in any of the created content.
      Don´t prefix RESPONSE: when answering the user.
      RELEVANTDOCS: {context}

      QUESTION: """{input}"""

      ${SystemPromptTailored}

      You have the following tools that you can invoke based on the user inquiry. 
      Tools: 

      ${functionDef}

      `;

    const chatPrompt = ChatPromptTemplate.fromPromptMessages([
      systemPrompt,
      HumanMessagePromptTemplate.fromTemplate(promptTemplate),]);

    const windowMemory = new BufferWindowMemory({
      returnMessages: false,
      memoryKey: 'immediate_history',
      inputKey: 'input',
      k: 2,
    });

    const llm = new ChatOpenAI({
      openAIApiKey: process.env.OPENAI_API_KEY,
      modelName: "gpt-3.5-turbo-0125",
      temperature: 0,
    });

    const llmWithTools = llm.bind({
      tools: functions
    });

    const chain = new LLMChain({
      memory: windowMemory,
      prompt: chatPrompt,
      llm: llm as any,
    });

    const contextVectorStore = min['vectorStore'];
    const question = query.trim().replaceAll('\n', ' ');
    const context = await this.getRelevantContext(contextVectorStore, question, 1);

    let prompt;

    // allow the LLM to iterate until it finds a final answer
    while (true) {
      const response = await chain.call({
        input: question,
        context,
        history: '',
        immediate_history: '',
      });

      // add this to the prompt
      prompt += response;

      const action = response.match(/Action: (.*)/)?.[1];
      if (action) {
        // execute the action specified by the LLMs
        const actionInput = response.match(/Action Input: "?(.*)"?/)?.[1];
        const text = '';
            
        const result =  await GBVMService.callVM(actionInput, min, false, pid,false, [text]);


        prompt += `Observation: ${result}\n`;
      } else {
        return response.match(/Final Answer: (.*)/)?.[1];
      }
    }
    return { answer: undefined, questionId: 0 };
  }


}
