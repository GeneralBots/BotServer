// import * as React from "react";
// import Header from "./Header";
// import HeroList, { HeroListItem } from "./HeroList";
// import Progress from "./Progress";
// import "../../../assets/icon-16.png";
// import "../../../assets/icon-32.png";
// import "../../../assets/icon-80.png";
// import $ from "jquery";

// export interface AppProps {
//   title: string;
//   isOfficeInitialized: boolean;
// }

// export interface AppState {
//   listItems: HeroListItem[];
//   mode: number;
//   conversationText: string;
//   scope: string;
//   state: number;
//   stateInfo: string;
//   inputText: string;
//   messages: string;
// }

// export default class App extends React.Component<AppProps, AppState> {
//   constructor(props, context) {
//     super(props, context);
//     this.state = {
//       mode: 0,
//       listItems: [],
//       conversationText: "",
//       scope: "",
//       state: 0,
//       stateInfo: "",
//       messages: "",
//       inputText: "",
//     };
//   }

//   botId = "dev-rodriguez22";
//   botKey = "starter";
//   host = "https://tender-yak-44.telebit.io";
//   breakpointsMap = {};

//   componentDidMount() {
//     this.setState({
//       listItems: [
//         {
//           icon: "Ribbon",
//           primaryText: "Office integration to Bots",
//         },
//         {
//           icon: "Unlock",
//           primaryText: "Unlock features of General Bots",
//         },
//         {
//           icon: "Design",
//           primaryText: "Create your Bots using BASIC",
//         },
//       ],
//     });
//   }

//   context = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/getContext`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function (item) {
//         console.log("GBWord Add-in: context OK.");
//         const line = item.line;

//         Word.run(async (context) => {
//           var paragraphs = context.document.body.paragraphs;
//           paragraphs.load("$none");
//           await context.sync();
//           for (let i = 0; i < paragraphs.items.length; i++) {
//             const paragraph = paragraphs.items[i];

//             context.load(paragraph, ["text", "font"]);
//             paragraph.font.highlightColor = null;

//             if (i === line) {
//               paragraph.font.highlightColor = "Yellow";
//             }
//           }
//           await context.sync();
//         });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;

//         console.log(textStatus);
//       });
//   };

//   setExecutionLine = async (line) => {
//     Word.run(async (context) => {
//       var paragraphs = context.document.body.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();
//       for (let i = 0; i < paragraphs.items.length; i++) {
//         const paragraph = paragraphs.items[i];

//         context.load(paragraph, ["text", "font"]);
//         paragraph.font.highlightColor = null;

//         if (i === line) {
//           paragraph.font.highlightColor = "Yellow";
//         }
//       }
//       await context.sync();
//     });
//   };

//   breakpoint = async () => {
//     let line = 0;

//     Word.run(async (context) => {
//       let selection = context.document.getSelection();
//       selection.load();

//       await context.sync();

//       console.log("Empty selection, cursor.");

//       const paragraph = selection.paragraphs.getFirst();
//       paragraph.select();
//       context.load(paragraph, ["text", "font"]);

//       var paragraphs = context.document.body.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();

//       for (let i = 0; i < paragraphs.items.length; i++) {
//         const paragraph1 = paragraphs.items[i];

//         if (paragraph1 === paragraph) {
//           line = i + 1;
//           paragraph.font.highlightColor = "Orange";
//         }
//       }

//       return context.sync();
//     });

//     const url = `${this.host}/api/v3/${this.botId}/dbg/setBreakpoint`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey, line },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: breakpoint OK.");
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;

//         console.log(textStatus);
//       });
//   };

//   refactor = async () => {
//     let line = 0;

//     let change = 'ssssssssssssssssssss';

//     Word.run(async (context) => {
//       let selection = context.document.getSelection();
//       selection.load();

//       await context.sync();

//       var paragraphs = selection.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();
//       let code = '';
//       for (let i = 0; i < paragraphs.items.length; i++) {

//         const paragraph = paragraphs.items[i];
//         context.load(paragraph, ["text", "font"]);
//         code += paragraph.text;
//       }

//       const url = `${this.host}/api/v3/${this.botId}/dbg/refactor`;

//       $.ajax({
//         data: { botId: this.botId, code: code, change: change },
//         url: url,
//         dataType: "json",
//         method: "POST",
//       })
//         .done(async function (data) {

//           Word.run(async (context) => {
//             var selectedRange = context.document.getSelection();
//             context.load(selectedRange, "text");
//             selectedRange.text = data;

//             await context.sync();
//           });
//         })
//         .fail(function (jqXHR, textStatus, errorThrown) {
//           console.log(textStatus);
//         });

//       return context.sync();
//     });

//   };

//   resume = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/resume`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: resume OK.");
//         this.setState({ mode: 1 });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;
//         console.log(textStatus);
//       });
//   };

//   step = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/step`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: step OK.");
//         this.setState({ mode: 2 });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;
//         console.log(textStatus);
//       });
//   };

//   stop = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/stop`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: stop OK.");
//         this.setState({ mode: 0 });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;
//         console.log(textStatus);
//       });
//   };

//   sendMessage = async (args) => {
//     if (args.keyCode === 13) {
//       const text = args.target.value;
//       const url = `${this.host}/api/v3/${this.botId}/dbg/sendMessage`;

//       $.ajax({
//         data: { botId: this.botId, botKey: this.botKey, text: text },
//         url: url,
//         dataType: "json",
//         method: "POST",
//       })
//         .done(function () {
//           console.log("GBWord Add-in: sendMessage OK.");
//           args.target.value = "";
//         })
//         .fail(function (jqXHR, textStatus, errorThrown) {
//           let x = jqXHR,
//             y = errorThrown;
//           console.log(textStatus);
//         });
//     }
//   };

//   waitFor = (delay) => new Promise((resolve) => setTimeout(resolve, delay));

//   refresh = async () => {
//     const context = await this.context();

//     this.setState({
//       conversationText: context['conversationText'],
//       state: context['state'],
//       messages: context['messages'],
//       scope: context['scope'],
//       mode: context['state']
//     });
//     await this.waitFor(3000);
//     await this.refresh();
//   };


//   debug = async () => {
//     if (this.state.mode === 0) {
//       const url = `${this.host}/api/v3/${this.botId}/dbg/start`;

//       $.ajax({
//         data: { botId: this.botId, botKey: this.botKey, scriptName: "auto" },
//         url: url,
//         dataType: "json",
//         method: "POST",
//       })
//         .done(function () {
//           console.log("GBWord Add-in: debug OK.");
//           this.state.mode = 1;
//         })
//         .fail(function (jqXHR, textStatus, errorThrown) {
//           let x = jqXHR,
//             y = errorThrown;
//           console.log(textStatus);
//         });
//     } else if (this.state.mode === 2) {
//       this.resume();
//     }

//     await this.refresh();
//   };

//   formatCode = async () => {
//     return Word.run(async (context) => {
//       var paragraphs = context.document.body.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();
//       for (let i = 0; i < paragraphs.items.length; i++) {
//         const paragraph = paragraphs.items[i];
//         context.load(paragraph, ["text", "font"]);
//         paragraph.font.highlightColor = null;

//         const words = paragraph.split([" "], true /* trimDelimiters*/, true /* trimSpaces */);
//         words.load(["text", "font"]);
//         await context.sync();
//         var boldWords = [];
//         for (var j = 0; j < words.items.length; ++j) {
//           var word = words.items[j];
//           if (word.text === "TALK" && j == 0) boldWords.push(word);
//           if (word.text === "HEAR" && j == 0) boldWords.push(word);
//           if (word.text === "SAVE" && j == 0) boldWords.push(word);
//           if (word.text === "FIND" && j == 3) boldWords.push(word);
//           if (word.text === "OPEN" && j == 0) boldWords.push(word);
//           if (word.text === "WAIT" && j == 0) boldWords.push(word);
//           if (word.text === "SET" && j == 0) boldWords.push(word);
//           if (word.text === "CLICK" && j == 0) boldWords.push(word);
//           if (word.text === "MERGE" && j == 0) boldWords.push(word);
//           if (word.text === "IF" && j == 0) boldWords.push(word);
//           if (word.text === "THEN" && j == 0) boldWords.push(word);
//           if (word.text === "ELSE" && j == 0) boldWords.push(word);
//           if (word.text === "END" && j == 0) boldWords.push(word);
//           if (word.text === "TWEET" && j == 0) boldWords.push(word);
//           if (word.text === "HOVER" && j == 0) boldWords.push(word);
//           if (word.text === "PRESS" && j == 0) boldWords.push(word);
//           if (word.text === "DO" && j == 0) boldWords.push(word);
//         }
//         for (var j = 0; j < boldWords.length; ++j) {
//           boldWords[j].font.color = "blue";
//           boldWords[j].font.bold = true;
//         }
//       }
//       await context.sync();
//     });
//   };

//   render() {
//     const { title, isOfficeInitialized } = this.props;

//     if (!isOfficeInitialized) {
//       return (
//         <Progress title={title} logo="assets/logo-filled.png" message="Please sideload your addin to see app body." />
//       );
//     }

//     return (
//       <div className="ms-welcome">
//         <Header logo="assets/logo-filled.png" title={this.props.title} message="Welcome" />
//         &nbsp;
//         <a onClick={this.formatCode} href="#">
//           <i className={`ms-Icon ms-Icon--DocumentApproval`} title="Format"></i>
//           &nbsp;Format
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.debug} href="#">
//           <i className={`ms-Icon ms-Icon--AirplaneSolid`} title="Run"></i>
//           &nbsp; Run
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.stop} href="#">
//           <i className={`ms-Icon ms-Icon--StopSolid`} title="Stop"></i>
//           &nbsp; Stop
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.step} href="#">
//           <i className={`ms-Icon ms-Icon--Next`} title="Step Over"></i>
//           &nbsp; Step
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.breakpoint} href="#">
//           <i className={`ms-Icon ms-Icon--DRM`} title="Set Breakpoint"></i>
//           &nbsp; Break
//         </a>
//         <br />
//         <br />
//         <div>Status: {this.state.stateInfo} </div>
//         <br />
//         <div>Bot Messages:</div>
//         <textarea title="Bot Messages" value={this.state.conversationText} readOnly={true}></textarea>
//         <br />
//         <textarea
//           title="Message"
//           readOnly={false}
//           onKeyDown={this.sendMessage}
//         ></textarea>
//         <div>Variables:</div>
//         <div>{this.state.scope} </div>
//         <HeroList message="Discover what General Bots can do for you today!!" items={this.state.listItems}></HeroList>
//       </div>
//     );
//   }
// }
