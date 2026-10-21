import "./style.css";

const app = document.querySelector<HTMLElement>("#app");

if (app) {
  const title = document.createElement("h1");
  title.textContent = "echodorsal";
  const message = document.createElement("p");
  message.textContent = "Assistente de projetos por voz";
  app.append(title, message);
}
