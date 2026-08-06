import {useEffect, useState} from "react";
import {HeaderContainer, Loading} from "@carbon/react";
import {listNamespaces} from "./hooks/api/use-namespaces";
import {ensureAuthenticated} from "./utils/auth";
import "./App.scss";

import Header from "./components/Header";
import SideNav from "./components/SideNav";
import Content from "./components/Content";

function App() {
  const [authReady, setAuthReady] = useState(false);

  useEffect(() => {
    ensureAuthenticated().then(() => {
      setAuthReady(true);
    });
  }, []);

  if (!authReady) {
    return <Loading withOverlay />;
  }

  listNamespaces();

  return (
    <HeaderContainer
      render={({ isSideNavExpanded, onClickSideNavExpand }) => (
        <>
          <Header isSideNavExpanded={isSideNavExpanded} onClickSideNavExpand={onClickSideNavExpand}/>
          <SideNav isSideNavExpanded={isSideNavExpanded} onClickSideNavExpand={onClickSideNavExpand}/>
          <Content/>
        </>
      )}
    />
  );
}

export default App;
