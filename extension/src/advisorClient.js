(function harmoniesAdvisorClientModule() {
  function createAdvisorClient() {
    return {
      async getRecommendation(gamedatas) {
        return window.HarmoniesMockAdvisor.recommend(gamedatas);
      },
    };
  }

  window.HarmoniesAdvisorClient = { createAdvisorClient };
})();
