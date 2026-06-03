(function harmoniesAdvisorClientModule() {
  function createAdvisorClient() {
    return {
      async getRecommendation(gamedatas) {
        const snapshot = window.HarmoniesBgaNormalizer.normalizeGamedatas(gamedatas);
        return window.HarmoniesMockAdvisor.recommend(snapshot);
      },
    };
  }

  window.HarmoniesAdvisorClient = { createAdvisorClient };
})();
