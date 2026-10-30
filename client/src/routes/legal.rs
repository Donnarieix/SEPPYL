use dioxus::prelude::*;
use super::Route;

// These values intentionally have NO fictional fallback identity.
// Supply the details of the actual site publisher and host during the production build.
const PUBLISHER:&str=match option_env!("SEPPYL_LEGAL_PUBLISHER"){
    Some(value)=>value,None=>"À renseigner avant la mise en ligne",
};
const PUBLISHER_ADDRESS:&str=match option_env!("SEPPYL_LEGAL_ADDRESS"){
    Some(value)=>value,None=>"À renseigner avant la mise en ligne",
};
const PUBLISHER_PHONE:&str=match option_env!("SEPPYL_LEGAL_PHONE"){
    Some(value)=>value,None=>"À renseigner avant la mise en ligne",
};
const HOST:&str=match option_env!("SEPPYL_LEGAL_HOST"){
    Some(value)=>value,None=>"À renseigner avant la mise en ligne",
};
const HOST_ADDRESS:&str=match option_env!("SEPPYL_LEGAL_HOST_ADDRESS"){
    Some(value)=>value,None=>"À renseigner avant la mise en ligne",
};
const HOST_PHONE:&str=match option_env!("SEPPYL_LEGAL_HOST_PHONE"){
    Some(value)=>value,None=>"À renseigner avant la mise en ligne",
};
const CONTROLLER:&str=match option_env!("SEPPYL_LEGAL_DATA_CONTROLLER"){
    Some(value)=>value,None=>"À déterminer selon l'organisation responsable des traitements",
};
const RETENTION:&str=match option_env!("SEPPYL_LEGAL_RETENTION"){
    Some(value)=>value,None=>"Durées à définir et à publier avant le lancement",
};
const DIRECTOR:&str=match option_env!("SEPPYL_LEGAL_DIRECTOR"){
    Some(value)=>value,None=>"À renseigner avant la mise en ligne",
};

/// The release displayed in the UI follows the package's major.minor fields.
pub fn version()->String {
    let version=env!("CARGO_PKG_VERSION");
    let mut parts=version.split('.');
    format!("v{}.{}",parts.next().unwrap_or("0"),parts.next().unwrap_or("1"))
}

#[component]
pub fn SiteFooter()->Element {
    rsx! {
        footer{class:"site-footer",role:"contentinfo",
            div{class:"site-footer-inner",
                div{class:"site-footer-brand",
                    strong{"SEPPYL"}
                    span{"Gestion des entraînements et des équipes"}
                }
                nav{class:"site-footer-nav","aria-label":"Informations et assistance",
                    a{href:"/mentions-legales","Mentions légales"}
                    a{href:"/confidentialite","Confidentialité"}
                    a{href:"/cgu","CGU"}
                    a{href:"mailto:seppyl@donnarieix.fr","Contact"}
                }
                div{class:"site-footer-meta",
                    span{class:"site-footer-version",title:"Version de SEPPYL","{version()}"}
                    a{href:"https://github.com/Donnarieix",target:"_blank",
                        rel:"noopener noreferrer", "Développé par Donnarieix"}
                }
            }
        }
    }
}

#[component]
fn LegalHeading(title:String, description:String)->Element {
    rsx! {
        div{class:"legal-heading",
            Link{class:"legal-return",to:Route::PublicDirectory{},"← Retour à SEPPYL"}
            span{class:"legal-eyebrow","Informations légales"}
            h1{"{title}"}
            p{"{description}"}
        }
    }
}

#[component]
pub fn LegalNotice()->Element {
    rsx! {
        document::Title{"Mentions légales | SEPPYL"}
        document::Link{rel:"canonical",href:"https://seppyl.donnarieix.fr/mentions-legales"}
        document::Meta{name:"description",content:"Informations sur l'éditeur, l'hébergement et les droits associés à SEPPYL."}
        main{class:"page legal-page",
            LegalHeading{
                title:"Mentions légales",
                description:"Identification de l'éditeur et informations relatives à l'hébergement du service."
            }
            article{class:"legal-article",
                section{
                    h2{"Édition du service"}
                    p{"SEPPYL est une application de gestion d'entraînements sportifs accessible à l'adresse seppyl.donnarieix.fr."}
                    dl{
                        dt{"Éditeur du service"} dd{"{PUBLISHER}"}
                        dt{"Adresse de l'éditeur"} dd{"{PUBLISHER_ADDRESS}"}
                        dt{"Téléphone"} dd{"{PUBLISHER_PHONE}"}
                        dt{"Directeur de la publication"} dd{"{DIRECTOR}"}
                        dt{"Adresse électronique"} dd{
                            a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                        }
                    }
                }
                section{
                    h2{"Hébergement"}
                    p{"Les informations légales de l'hébergeur de l'infrastructure doivent correspondre à l'installation réellement utilisée."}
                    dl{
                        dt{"Hébergeur / responsable de l'infrastructure"} dd{"{HOST}"}
                        dt{"Adresse de l'hébergeur"} dd{"{HOST_ADDRESS}"}
                        dt{"Téléphone de l'hébergeur"} dd{"{HOST_PHONE}"}
                    }
                }
                section{
                    h2{"Propriété intellectuelle"}
                    p{"L'application, son code et ses éléments graphiques sont protégés selon les droits et licences applicables. Les logos, textes, illustrations et contenus déposés par les clubs restent sous la responsabilité de leurs titulaires. Leur réutilisation nécessite l'autorisation des ayants droit, sauf exception légale ou licence le permettant."}
                    p{"Développement : "}
                    a{href:"https://github.com/Donnarieix",target:"_blank",rel:"noopener noreferrer","Donnarieix sur GitHub"}
                }
                section{
                    h2{"Contact"}
                    p{"Pour signaler un problème technique, faire une demande concernant vos données ou contacter l'éditeur : "}
                    a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                }
            }
        }
    }
}

#[component]
pub fn PrivacyPolicy()->Element {
    rsx! {
        document::Title{"Politique de confidentialité | SEPPYL"}
        document::Link{rel:"canonical",href:"https://seppyl.donnarieix.fr/confidentialite"}
        document::Meta{name:"description",content:"Comment SEPPYL traite les données des comptes, clubs, équipes et joueuses, et comment exercer vos droits."}
        main{class:"page legal-page",
            LegalHeading{
                title:"Politique de confidentialité",
                description:"Informations relatives aux traitements de données effectués dans le cadre de SEPPYL."
            }
            article{class:"legal-article",
                section{
                    h2{"1. Responsable du traitement"}
                    p{"L'identité du responsable des traitements liés à la plateforme et la répartition des responsabilités avec les clubs doivent être précisées avant l'ouverture du service."}
                    p{"Responsable déclaré : {CONTROLLER}."}
                    p{"Pour toute question concernant vos données : "}
                    a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                }
                section{
                    h2{"2. Données concernées"}
                    ul{
                        li{"Comptes : identifiant, mot de passe stocké sous forme d'empreinte, rôle, habilitations, sessions et préférences d'affichage."}
                        li{"Gestion sportive : identité et numéro des joueuses, affiliations aux clubs et équipes, séances, présences, évaluations par activité et notes."}
                        li{"Contenus : descriptions, documents sportifs, tactiques et images ajoutés par les utilisateurs autorisés."}
                        li{"Sécurité et traçabilité : auteur des modifications, action réalisée, horodatage et journaux techniques."}
                        li{"Historique des versions : version consultée et date de consultation par compte."}
                    }
                }
                section{
                    h2{"3. Finalités et bases juridiques"}
                    p{"Les données servent à fournir le service de gestion sportive, permettre la connexion, gérer les accès, synchroniser les contributions, assurer la sécurité et répondre aux demandes d'assistance."}
                    p{"Selon les traitements et l'organisme responsable, les bases légales possibles incluent l'exécution d'un service demandé, l'intérêt légitime pour sa sécurité ou le respect d'obligations légales. La base légale exacte de chaque traitement, notamment des informations de joueuses, doit être documentée par son responsable avant la mise en production."}
                }
                section{
                    h2{"4. Destinataires et visibilité"}
                    p{"Les données privées des joueuses et des entraînements sont destinées aux utilisateurs qui disposent des habilitations correspondantes. Les Mainteneurs et Administrateurs disposent d'un accès de gestion selon leurs rôles. Les clubs ayant activé leur visibilité rendent publics les noms, descriptions et logos des clubs et équipes, mais pas les fiches des joueuses ni leurs évaluations."}
                    p{"L'hébergeur et les éventuels prestataires nécessaires au fonctionnement du service peuvent être amenés à traiter certaines données. Leur identité et les éventuels transferts hors de l'Espace économique européen doivent être vérifiés et documentés par le responsable du traitement."}
                    p{"Lorsqu'un club choisit de renseigner un logo hébergé sur un site tiers, l'affichage de cette image peut déclencher une requête du navigateur vers le serveur tiers, susceptible de recevoir certaines données techniques comme l'adresse IP. Les responsables du service doivent en vérifier les implications avant publication."}
                }
                section{
                    h2{"5. Conservation"}
                    p{"{RETENTION}."}
                    p{"Les données ne doivent pas être conservées indéfiniment : la politique de conservation doit distinguer les comptes, les dossiers des joueuses, les séances, les images, les journaux de sécurité et les sauvegardes, puis prévoir leur suppression ou anonymisation à l'échéance."}
                }
                section{
                    h2{"6. Sessions et traceurs"}
                    p{"SEPPYL utilise actuellement le stockage local du navigateur pour conserver le jeton de session ainsi que les préférences nécessaires au fonctionnement de l'application. Ces données ne sont pas utilisées à des fins de publicité. Il n'est pas prévu d'installer des traceurs publicitaires ou de mesure d'audience nécessitant un consentement dans cette version."}
                    p{"Si des traceurs facultatifs sont ajoutés ultérieurement, une information et, si nécessaire, un recueil de consentement préalable devront être mis en place."}
                }
                section{
                    h2{"7. Vos droits"}
                    p{"Selon les règles applicables, vous pouvez demander l'accès, la rectification, l'effacement, la limitation et, dans certains cas, la portabilité de vos données ou vous opposer à leur traitement. Vous pouvez aussi introduire une réclamation auprès de la CNIL."}
                    p{"Pour exercer vos droits ou demander la suppression d'un compte : "}
                    a{href:"mailto:seppyl@donnarieix.fr?subject=SEPPYL%20-%20Donn%C3%A9es%20personnelles","seppyl@donnarieix.fr"}
                    p{"En présence de données relatives à des joueuses mineures, le club concerné doit informer les personnes concernées et, selon la situation, leurs représentants légaux."}
                }
                section{
                    h2{"8. Sécurité"}
                    p{"Les accès sont protégés par authentification et autorisations liées aux rôles. Les responsables du service doivent veiller à la protection des comptes, à la sécurité de l'hébergement, aux sauvegardes et à la gestion des incidents. Aucune mesure technique ne garantit à elle seule l'absence totale de risque."}
                }
            }
        }
    }
}

#[component]
pub fn TermsOfUse()->Element {
    rsx! {
        document::Title{"Conditions générales d'utilisation | SEPPYL"}
        document::Link{rel:"canonical",href:"https://seppyl.donnarieix.fr/cgu"}
        document::Meta{name:"description",content:"Règles d'utilisation de la plateforme SEPPYL pour les clubs, entraîneurs et utilisateurs."}
        main{class:"page legal-page",
            LegalHeading{
                title:"Conditions générales d'utilisation",
                description:"Cadre d'utilisation du service de préparation et de suivi des entraînements sportifs."
            }
            article{class:"legal-article",
                section{
                    h2{"1. Objet"}
                    p{"SEPPYL permet aux clubs et aux entraîneurs habilités d'organiser des séances, de gérer des activités et effectifs, de suivre des présences et des évaluations, et d'utiliser des outils tactiques. L'accès au service implique le respect des présentes règles d'utilisation."}
                }
                section{
                    h2{"2. Comptes et accès"}
                    p{"Les utilisateurs doivent préserver la confidentialité de leurs identifiants, ne pas partager leur compte et prévenir le responsable en cas de suspicion d'accès non autorisé. Les fonctions disponibles dépendent des habilitations accordées. Un compte créé n'obtient pas automatiquement accès aux données privées d'un club."}
                }
                section{
                    h2{"3. Utilisation autorisée"}
                    p{"Les utilisateurs s'engagent à renseigner uniquement les données nécessaires et exactes, à ne pas porter atteinte aux droits d'autrui et à ne pas tenter de contourner les contrôles d'accès. L'import d'images, de logos ou de textes suppose de disposer des droits nécessaires pour leur utilisation."}
                    p{"Les informations relatives aux joueuses, particulièrement lorsqu'elles sont mineures, doivent être saisies et consultées par des personnes légitimement habilitées."}
                }
                section{
                    h2{"4. Responsabilité des clubs"}
                    p{"Chaque club reste responsable de l'organisation de ses activités, de l'attribution des droits de ses membres et de l'information de ses joueuses concernant la collecte et l'utilisation des données, dans les limites de la répartition des responsabilités à définir avec l'éditeur de la plateforme."}
                }
                section{
                    h2{"5. Disponibilité et évolution"}
                    p{"Le service peut évoluer pour des raisons de maintenance, de sécurité ou d'amélioration. Les utilisateurs doivent conserver leurs documents importants en dehors de l'application lorsque nécessaire. Aucune garantie de disponibilité permanente n'est annoncée."}
                }
                section{
                    h2{"6. Données personnelles"}
                    p{"Le traitement des données personnelles est présenté dans la "}
                    Link{to:Route::PrivacyPolicy{},"politique de confidentialité"}
                    p{"Les demandes liées aux données ou aux difficultés d'utilisation peuvent être transmises à "}
                    a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                }
                section{
                    h2{"7. Contact et signalements"}
                    p{"Pour signaler un contenu problématique, une erreur ou une difficulté technique, contactez "}
                    a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                }
            }
        }
    }
}
