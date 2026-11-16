-- bld.market@gunpowder: Early modern market (mesh.market_gunpowder)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.market_gunpowder','Early modern market');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.market_gunpowder';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('yard','plinth','mat.ashlar',0,'','{"w":3.7,"d":3.7,"h":0.06,"steps":1,"tint":"#c8c0ae"}'),
  ('arcade','arch','mat.ashlar',0,'','{"x":-0.9,"y":0.06,"z":-0.47,"w":0.9,"h":1,"t":0.16,"span":0.58,"spring":0.56,"repeat":3,"dx":0.9}'),
  ('arcade_back','wall','mat.ashlar',0,'','{"y":0.06,"z":-1.6,"w":2.7,"h":1,"t":0.14}'),
  ('arcade_sides','box','mat.ashlar',0,'','{"x":-1.28,"y":0.06,"z":-1.04,"w":0.14,"d":1,"h":1,"repeat":2,"dx":2.56}'),
  ('upper','box','mat.ochre_plaster',0,'','{"y":1.06,"z":-1.04,"w":2.76,"d":1.28,"h":0.78,"windows":3,"window_y":0.22,"window_w":0.2,"window_h":0.36,"side_windows":1,"opening":"#2c3642"}'),
  ('roof','hip_roof','mat.pantile',0,'','{"y":1.84,"z":-1.04,"w":2.76,"d":1.28,"pitch":40,"overhang":0.14,"t":0.08}'),
  ('roof_cap','hip_roof','mat.cloth',1,'','{"y":2.263,"z":-1.04,"w":2,"d":0.52,"pitch":40,"overhang":0,"t":0.03}'),
  ('bell','box','mat.ashlar',0,'','{"y":2.3,"z":-1.04,"w":0.26,"d":0.26,"h":0.36,"far":0}'),
  ('bell_roof','dome','mat.copper',0,'','{"y":2.66,"z":-1.04,"w":0.34,"h":0.24,"segments":6,"far":0}'),
  ('stall_w','include',NULL,0,'','{"mesh":"mesh.stall","x":-1.3,"y":0.06,"z":0.6,"yaw":90,"scale":0.85}'),
  ('stall_e','include',NULL,0,'','{"mesh":"mesh.stall","x":1.3,"y":0.06,"z":0.6,"yaw":-90,"scale":0.85}'),
  ('stall_s','include',NULL,0,'','{"mesh":"mesh.stall","y":0.06,"z":1.3,"yaw":180,"scale":0.85}'),
  ('pump','box','mat.ashlar',0,'','{"y":0.06,"z":0.35,"w":0.3,"d":0.3,"h":0.7,"taper":0.15,"far":0}'),
  ('pump_top','dome','mat.brass',0,'','{"y":0.76,"z":0.35,"w":0.26,"h":0.16,"segments":6,"far":0}'),
  ('barrels','cylinder','mat.timber',0,'','{"x":-0.55,"y":0.06,"z":-0.1,"w":0.26,"h":0.36,"segments":7,"repeat":3,"dx":0.32,"tint":"#c8a888","far":0}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.market_gunpowder'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='bld.market'), NULL, (SELECT id FROM def WHERE key='age.gunpowder'), (SELECT id FROM def WHERE key='mesh.market_gunpowder'), (SELECT id FROM def WHERE key='mat.ochre_plaster'), 100);
