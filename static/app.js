 function app() {
  return {
    stage: "username",
    username: "",
    error: "",
    pollLink: "",
    copiedMsg: "",
    newPoll: {
      username: "",
      title: "",
      voting_time: 60,
      options: [{ title: "" }],
      is_multi: false,
    },

    async submitUsername() {
      this.error = "";
      if (this.username.length < 3) return this.error = "Invalid username.";
      this.newPoll.username = this.username;
      this.stage = "create";
    },

    async createPoll() {
      this.error = "";
      try {
        const res = await fetch("http://localhost:8080/", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.newPoll),
        });
        if (!res.ok) throw new Error(await res.text());
        const uuid = await res.text();
        this.pollLink = `http://localhost:8080/${uuid}`;
        this.stage = "created";
      } catch (e) {
        this.error = e.message || "Poll creation failed.";
      }
    },

    async copyLink() {
      await navigator.clipboard.writeText(this.pollLink);
      this.copiedMsg = "Link copied!";
      setTimeout(() => this.copiedMsg = "", 2000);
    },
  };
};


document.addEventListener('alpine:init', () => {
  Alpine.data('app', window.app);
});
