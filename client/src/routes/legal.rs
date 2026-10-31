use dioxus::prelude::*;
use super::Route;

// These values intentionally have NO fictional fallback identity.
// Supply the details of the actual site publisher and host during the production build.
// These are the only publication details that cannot be inferred from the
// source code or domain name. They are supplied privately at build time.
// A production build must fail its preflight if the identity is absent.
const PUBLISHER:Option<&str>=option_env!("SEPPYL_LEGAL_PUBLISHER");
const PUBLISHER_ADDRESS:Option<&str>=option_env!("SEPPYL_LEGAL_ADDRESS");
const PUBLISHER_PHONE:Option<&str>=option_env!("SEPPYL_LEGAL_PHONE");
const DIRECTOR:Option<&str>=option_env!("SEPPYL_LEGAL_DIRECTOR");


/// The release displayed in the UI follows the package's major.minor fields.
pub fn version()->String {
    format!("v{}",env!("CARGO_PKG_VERSION"))
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
        document::Meta{name:"description",content:"Identification de l'éditeur de SEPPYL, conditions d'hébergement à domicile et contact."}
        main{class:"page legal-page",
            LegalHeading{
                title:"Mentions légales",
                description:"Informations sur l'édition et le fonctionnement du service SEPPYL."
            }
            article{class:"legal-article",
                section{
                    h2{"Présentation et éditeur"}
                    p{"SEPPYL (https://seppyl.donnarieix.fr) est un service gratuit de gestion d'entraînements et d'équipes sportives, publié à titre non professionnel par une personne physique sous le nom de projet Donnarieix."}
                    p{"Donnarieix est une identité de projet et une organisation GitHub : il ne s'agit pas d'une entreprise ou d'une association déclarée au titre de l'édition de SEPPYL."}
                    dl{
                        dt{"Éditeur"}
                        dd{
                            if let Some(name)=PUBLISHER{"{name}"}
                            else{"Personne physique, édition à titre non professionnel (Donnarieix)"}
                        }
                        if let Some(name)=DIRECTOR{
                            dt{"Responsable de la publication"} dd{"{name}"}
                        }
                        if let Some(address)=PUBLISHER_ADDRESS{
                            dt{"Adresse postale"} dd{"{address}"}
                        }
                        if let Some(phone)=PUBLISHER_PHONE{
                            dt{"Téléphone"} dd{"{phone}"}
                        }
                        dt{"Adresse électronique"} dd{
                            a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                        }
                    }
                }
                section{
                    h2{"Hébergement et prestataires"}
                    p{"L'application et ses données sont hébergées sur un serveur privé administré par l'éditeur à son domicile, en France. Il ne s'agit pas d'un hébergement Web souscrit auprès d'o2switch."}
                    p{"o2switch intervient pour les services DNS liés au domaine donnarieix.fr et pour la messagerie utilisée par l'adresse de contact. Le fournisseur DNS ou de messagerie ne doit pas être confondu avec l'hébergeur de l'application."}
                    p{"Pour toute demande concernant le site et son hébergement : "}
                    a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                }
                section{
                    h2{"Contenus et propriété intellectuelle"}
                    p{"Le code, les interfaces et les éléments graphiques de SEPPYL sont soumis aux droits et aux licences qui leur sont applicables. Les contributeurs et les clubs restent responsables des droits d'utilisation des logos, images, descriptions et autres contenus qu'ils publient."}
                    p{"Le développement est présenté par l'organisation "}
                    a{href:"https://github.com/Donnarieix",target:"_blank",rel:"noopener noreferrer","Donnarieix sur GitHub"}
                    p{"Le crédit de développement ne désigne pas à lui seul l'éditeur juridiquement responsable du service."}
                }
                section{
                    h2{"Signaler un contenu ou contacter l'éditeur"}
                    p{"Pour contacter l'éditeur, demander le retrait d'un contenu, signaler un comportement illicite ou exercer des droits relatifs aux données personnelles, écrivez à "}
                    a{href:"mailto:seppyl@donnarieix.fr?subject=SEPPYL%20-%20Signalement","seppyl@donnarieix.fr"}
                    p{"Précisez l'URL ou l'élément concerné et le motif de votre demande. Les signalements seront examinés ; les contenus manifestement illicites pourront être retirés ou leur accès limité."}
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
        document::Meta{name:"description",content:"Traitements des données des comptes, clubs, équipes, joueuses et utilisateurs de SEPPYL."}
        main{class:"page legal-page",
            LegalHeading{
                title:"Politique de confidentialité",
                description:"Comment SEPPYL utilise et protège les informations que vous lui confiez."
            }
            article{class:"legal-article",
                section{
                    h2{"1. Qui traite les données ?"}
                    p{"SEPPYL est un service gratuit édité à titre non professionnel par une personne physique sous le nom Donnarieix. Pour le fonctionnement des comptes, l'authentification, la sécurité et la maintenance de l'application, l'éditeur détermine les moyens techniques et les finalités du traitement."}
                    p{"Pour les données sportives, les responsables de clubs et les personnes habilitées décident des joueuses, entraînements et informations qu'ils saisissent. Selon l'organisation de chaque club, celui-ci peut être responsable des traitements sportifs qu'il met en œuvre, SEPPYL intervenant comme outil technique. Les responsabilités doivent être précisées avec les clubs concernés, notamment lorsqu'ils sont constitués en associations."}
                    p{"Contact pour la confidentialité et l'exercice des droits : "}
                    a{href:"mailto:seppyl@donnarieix.fr?subject=SEPPYL%20-%20Protection%20des%20donn%C3%A9es","seppyl@donnarieix.fr"}
                }
                section{
                    h2{"2. Quelles données sont utilisées ?"}
                    ul{
                        li{"Comptes : identifiant, empreinte du mot de passe, rôle, rattachements, préférences et données nécessaires à la session."}
                        li{"Clubs et équipes : nom, présentation, logos, visibilité publique et composition des effectifs."}
                        li{"Joueuses : prénom, nom, numéro, affiliations sportives, présences et notes de performance associées aux entraînements et activités."}
                        li{"Travail sportif : entraînements, dates, exercices, descriptions, tactiques, images et matériel saisis par les membres autorisés."}
                        li{"Traçabilité : auteur d'une action, opération réalisée, date, périmètre concerné et résultats enregistrés dans le journal d'activité."}
                        li{"Historique des versions : version consultée et date d'acquittement du changelog par compte."}
                        li{"Support : informations volontairement communiquées par email, nécessaires à l'étude et au traitement de la demande."}
                    }
                    p{"SEPPYL n'a pas pour objet de collecter des informations médicales, des diagnostics ou des données de santé. Il est demandé aux utilisateurs de ne pas saisir de telles informations dans les champs libres, les notes ou les images."}
                }
                section{
                    h2{"3. Pourquoi ces données sont-elles traitées ?"}
                    p{"La gestion des comptes et des séances sert à fournir les fonctionnalités demandées par les utilisateurs du service gratuit. Les données nécessaires à l'identification, à l'administration et à la sécurisation des accès servent au fonctionnement et à la protection de la plateforme. Les journaux permettent de rechercher des erreurs, des abus et des incidents de sécurité."}
                    p{"Les traitements reposent, selon leur objet, sur l'exécution de la relation de service, sur l'intérêt légitime à sécuriser et administrer l'application, ou sur les obligations légales applicables. Les clubs déterminent la base légale appropriée à la gestion des informations sportives qu'ils collectent et doivent en informer les personnes concernées."}
                    p{"Aucune donnée n'est collectée dans un objectif de publicité ciblée ni vendue à des annonceurs."}
                }
                section{
                    h2{"4. Qui peut consulter les données ?"}
                    p{"Les comptes ne disposent que des accès autorisés par leurs rôles et leurs rattachements aux clubs ou aux équipes. Les administrateurs et mainteneurs de SEPPYL disposent des droits nécessaires à la gestion globale, notamment à la sécurité et au traitement des incidents."}
                    p{"Lorsqu'un gestionnaire rend un club public, ses nom, logo, description et informations d'équipes publiées sont consultables sans connexion. Les fiches individuelles des joueuses, leurs présences et leurs évaluations ne sont pas publiées dans cet annuaire dans la version actuelle."}
                    p{"Les données sont stockées sur le serveur privé de l'éditeur, en France. o2switch intervient pour le DNS et la messagerie de contact, pas pour le stockage principal des données applicatives. Des demandes d'assistance envoyées par email sont transmises au service de messagerie correspondant."}
                    p{"Certaines images peuvent être chargées depuis des adresses HTTPS choisies par les utilisateurs. Leur affichage peut contacter le serveur externe, qui reçoit alors des informations techniques liées à la requête, telles que l'adresse IP. L'éditeur ne peut garantir la localisation de ces serveurs tiers ; l'utilisation de logos téléversés directement sur SEPPYL permet d'éviter cet appel pour les images concernées."}
                }
                section{
                    h2{"5. Combien de temps les données sont-elles conservées ?"}
                    p{"Les comptes, autorisations et préférences sont utilisés tant que le compte est nécessaire au fonctionnement du service. Une demande de suppression peut être adressée à l'éditeur, sous réserve des informations qui doivent être conservées pour des raisons de sécurité ou d'obligation légale."}
                    p{"Les fiches de joueuses, les séances, présences, évaluations, images et contenus sportifs sont utilisés pendant la période où le club en a besoin pour organiser son activité. Lorsqu'ils ne sont plus nécessaires, leur retrait ou leur anonymisation doit être demandé ou organisé par les responsables habilités du club. Une révision est recommandée à chaque nouvelle saison sportive."}
                    p{"Les journaux d'activité sont utilisés pour la traçabilité et la sécurité du service et doivent faire l'objet d'une revue régulière afin de limiter leur durée de conservation au nécessaire. Les sauvegardes techniques suivent leur propre cycle de renouvellement et ne constituent pas une archive destinée à conserver indéfiniment des données supprimées de la base active."}
                    p{"L'application ne comporte pas encore de purge automatique couvrant l'ensemble de ces catégories : les demandes de retrait et les opérations de conservation sont traitées par l'éditeur et les gestionnaires concernés. Pour obtenir des précisions ou demander une suppression, écrivez à "}
                    a{href:"mailto:seppyl@donnarieix.fr?subject=SEPPYL%20-%20Suppression%20de%20donn%C3%A9es","seppyl@donnarieix.fr"}
                }
                section{
                    h2{"6. Sessions, stockage local et cookies"}
                    p{"Dans la version actuelle, le navigateur conserve le jeton de session dans son stockage local ainsi que certaines préférences d'affichage. Ces mécanismes sont utilisés pour maintenir la connexion et le fonctionnement de l'interface."}
                    p{"SEPPYL n'intègre actuellement ni régie publicitaire, ni outil d'analyse d'audience, ni traceur de ciblage marketing. Aucun bandeau de consentement publicitaire n'est donc prévu pour ces fonctions absentes. Si de tels services sont ajoutés, la présente politique et les mécanismes de consentement seront adaptés lorsque cela sera nécessaire."}
                    p{"Une intégration future avec un service d'authentification unique Donnarieix pourra modifier la gestion des sessions ; elle donnera lieu à une mise à jour de l'information avant sa mise en service."}
                }
                section{
                    h2{"7. Mineurs et informations sportives"}
                    p{"L'annuaire public peut être consulté par tout visiteur, y compris mineur. À ce jour, les comptes sont principalement destinés aux entraîneurs, gestionnaires et autres encadrants ; l'accès personnel des joueuses n'est pas une fonctionnalité spécifiquement prévue pour cette version."}
                    p{"Les effectifs peuvent comprendre des joueuses mineures. Les responsables des clubs doivent en informer les personnes concernées et, lorsque nécessaire, leurs représentants légaux, avant de saisir leurs données. La publication éventuelle de photographies, d'effectifs nominatifs ou d'autres données de mineures nécessitera une analyse et des autorisations adaptées avant l'activation de telles fonctionnalités."}
                }
                section{
                    h2{"8. Vos droits"}
                    p{"Vous pouvez, selon la situation et la base légale du traitement, demander l'accès aux données vous concernant, leur rectification, leur effacement, la limitation du traitement, vous opposer à certains usages ou demander leur portabilité."}
                    p{"Pour effectuer une demande : "}
                    a{href:"mailto:seppyl@donnarieix.fr?subject=SEPPYL%20-%20Exercice%20de%20mes%20droits","seppyl@donnarieix.fr"}
                    p{"Indiquez votre demande et les éléments permettant d'identifier le compte ou le club concerné, sans envoyer de mot de passe. Une vérification d'identité proportionnée pourra être demandée si nécessaire. Les demandes sont traitées dans les délais légaux applicables, normalement un mois, sous réserve des prolongations prévues par la réglementation."}
                    p{"Vous pouvez également déposer une réclamation auprès de la "}
                    a{href:"https://www.cnil.fr/fr/plaintes",target:"_blank",rel:"noopener noreferrer","CNIL"}
                }
                section{
                    h2{"9. Sécurité et évolution du service"}
                    p{"L'accès aux fonctionnalités privées dépend de l'authentification et des habilitations attribuées. Le service est administré sur une infrastructure privée, qui nécessite une gestion continue des correctifs, sauvegardes et accès. Aucun système ne peut garantir l'absence totale de risque."}
                    p{"En cas de changement majeur des fonctionnalités, de prestataires ou des traitements, cette politique sera mise à jour. Une information particulière pourra être fournie aux utilisateurs lorsque cela est nécessaire."}
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
        document::Meta{name:"description",content:"Conditions d'accès, règles de publication et responsabilités des utilisateurs de SEPPYL."}
        main{class:"page legal-page",
            LegalHeading{
                title:"Conditions générales d'utilisation",
                description:"Règles d'accès et d'utilisation de SEPPYL, application sportive gratuite."
            }
            article{class:"legal-article",
                section{
                    h2{"1. Objet du service"}
                    p{"SEPPYL est une application gratuite publiée à titre non professionnel par l'éditeur sous le nom de projet Donnarieix. Elle permet la gestion des clubs, équipes, effectifs, exercices, entraînements, présences, évaluations et schémas tactiques."}
                    p{"Le service n'est pas une prestation payante, et son utilisation n'emporte aucun abonnement ni engagement financier."}
                }
                section{
                    h2{"2. Inscription et accès aux clubs"}
                    p{"La consultation de l'annuaire des clubs publics est ouverte à tous, sans authentification. Il est possible de créer un compte, mais l'inscription ne donne pas automatiquement accès aux informations privées d'un club."}
                    p{"L'attribution des rôles, des clubs et des équipes relève des administrateurs et gestionnaires habilités. Dans sa version actuelle, le service est destiné principalement aux encadrants et responsables sportifs ; aucun espace personnel destiné aux joueuses n'est fourni."}
                    p{"L'utilisateur doit protéger ses identifiants, ne pas partager son compte et signaler tout accès suspect à "}
                    a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                }
                section{
                    h2{"3. Données et contenus publiés"}
                    p{"Les utilisateurs disposant des droits adaptés peuvent saisir des descriptions, notes, évaluations, fichiers, logos et autres contenus utiles à la gestion sportive. Ils s'engagent à ne publier que des contenus exacts dans la mesure du possible, licites, pertinents et respectueux d'autrui."}
                    p{"Les propos discriminatoires, racistes, menaçants, diffamatoires, les images illicites, les contenus portant atteinte à la vie privée et les contenus utilisés sans autorisation des titulaires des droits sont interdits."}
                    p{"Il est demandé de ne pas enregistrer de renseignements médicaux ou d'informations sensibles dans les commentaires, images et descriptions libres. Les clubs doivent être particulièrement vigilants au traitement des informations relatives aux mineures."}
                    p{"La personne qui ajoute un contenu garantit disposer des droits nécessaires à sa publication. Elle reste responsable de ses contributions et des informations saisies dans le cadre des droits qui lui sont accordés."}
                }
                section{
                    h2{"4. Données des clubs et responsabilités"}
                    p{"Chaque club ou personne disposant des autorisations de gestion décide des personnes rattachées à ses équipes, des activités renseignées et des contenus qui doivent rester privés ou être publiés."}
                    p{"Les données nominatives des joueuses et leurs évaluations restent privées dans cette version. Les réglages de visibilité ne concernent actuellement que la présentation publique des clubs et des équipes."}
                    p{"Il appartient aux responsables du club d'informer les joueuses concernées et, s'il y a lieu, leurs représentants légaux de la saisie de leurs informations et d'examiner les demandes relatives à ces données. L'éditeur de SEPPYL fournit l'outil et traite les demandes relevant de sa propre infrastructure et des comptes."}
                }
                section{
                    h2{"5. Signalement et intervention"}
                    p{"SEPPYL ne met pas en place de surveillance systématique préalable des contributions. Cela ne signifie pas que les contenus signalés resteront en ligne : tout contenu potentiellement illicite ou portant atteinte à des personnes peut être signalé à "}
                    a{href:"mailto:seppyl@donnarieix.fr?subject=SEPPYL%20-%20Signalement%20de%20contenu","seppyl@donnarieix.fr"}
                    p{"Merci de préciser le contenu, son emplacement et le motif du signalement. Après examen, l'éditeur peut retirer un contenu, suspendre un accès ou prendre toute mesure raisonnable pour prévenir une utilisation abusive, dans le respect des règles applicables."}
                }
                section{
                    h2{"6. Fonctionnement, disponibilité et évolution"}
                    p{"Le service est fourni gratuitement, sur une infrastructure auto-hébergée. Des interruptions peuvent survenir lors d'opérations techniques, de pannes, de mises à jour ou d'incidents indépendants de la volonté de l'éditeur."}
                    p{"Les utilisateurs sont encouragés à conserver leurs documents importants par un autre moyen lorsque cela est nécessaire. Des fonctionnalités peuvent évoluer, être suspendues ou être supprimées, sous réserve des obligations légales applicables."}
                }
                section{
                    h2{"7. Données personnelles"}
                    p{"Les informations sur les traitements de données personnelles, les destinataires, la conservation et les droits des personnes figurent dans la "}
                    Link{to:Route::PrivacyPolicy{},"politique de confidentialité"}
                    p{"Une demande d'accès, de rectification ou de suppression peut également être adressée à "}
                    a{href:"mailto:seppyl@donnarieix.fr?subject=SEPPYL%20-%20Donn%C3%A9es%20personnelles","seppyl@donnarieix.fr"}
                }
                section{
                    h2{"8. Propriété intellectuelle"}
                    p{"L'utilisation de SEPPYL ne transfère aucun droit de propriété intellectuelle sur son code, ses interfaces, son nom ou les contenus publiés par d'autres utilisateurs. La réutilisation de tout contenu demeure soumise aux licences et autorisations applicables."}
                }
                section{
                    h2{"9. Contact et évolution des conditions"}
                    p{"Pour toute question relative à l'utilisation du service, contactez "}
                    a{href:"mailto:seppyl@donnarieix.fr","seppyl@donnarieix.fr"}
                    p{"Ces conditions peuvent être adaptées à l'évolution des fonctionnalités et aux obligations applicables. Toute modification substantielle pourra être signalée dans l'application."}
                }
            }
        }
    }
}
